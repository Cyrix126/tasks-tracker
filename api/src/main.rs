use bincode::config::Configuration;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tasks_tracker_common::NewTask;
use tasks_tracker_common::Task;
use tasks_tracker_common::TaskStatus;

use authorize::is_authorized;
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header::ALLOW, HeaderMap, Method, StatusCode},
    response::{AppendHeaders, IntoResponse},
    routing::{delete, get, patch, post},
    Router,
};
use clap::Parser;
use tokio::{spawn, time::sleep};
use uuid::Uuid;

mod authorize;

#[derive(Parser)]
struct Args {
    token_create: String,
    token_admin: Option<String>,
    #[arg(default_value_t = 8000)]
    port: u16,
}

// Possible type of authorization
// admin token gives any privileges.
// the creation privilege always grant to View,Abort and Update privilege since the response include all three tokens.
enum ClientPrivilege {
    Creation,
    View(Uuid),
    Abort(Uuid),
    Update(Uuid),
    List,
}

#[derive(Clone)]
struct AppState {
    tasks: Arc<Mutex<Vec<Task>>>,
    token_admin: Option<String>,
    token_create: String,
    config_bincode: bincode::config::Configuration,
}

fn routes(state: AppState) -> Router {
    Router::new()
        .route("/tasks", get(list_tasks))
        .route("/tasks", post(create_task))
        .route("/tasks/:id", get(view_task))
        .route("/tasks/:id", patch(update_task))
        .route("/tasks/:id", delete(update_task))
        .with_state(state)
}

#[tokio::main]
async fn main() {
    // get envs for admin token and port number to listen to
    let args = Args::parse();

    // create the app struct
    let state = AppState {
        tasks: Arc::new(Mutex::new(Vec::new())),
        token_admin: args.token_admin,
        token_create: args.token_create,
        config_bincode: bincode::config::standard(),
    };

    // create routes
    let routes = routes(state);
    let adr = format!("127.0.0.1:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&adr).await.unwrap();
    axum::serve(listener, routes).await.unwrap();
}

async fn list_tasks(headers: HeaderMap, State(state): State<AppState>) -> impl IntoResponse {
    let authorized_status = is_authorized(&headers, &state, ClientPrivilege::List);
    if authorized_status != StatusCode::OK {
        return authorized_status.into_response();
    }
    let tasks = state.tasks.lock().unwrap().clone();
    // bitcode::serialize(&tasks[0])
    //     .expect("tasks are serializable so it should not panic")
    //     .into_response()
    bincode::encode_to_vec(tasks, state.config_bincode)
        .unwrap()
        .into_response()
    // bitcode::serialize(&3u32)
    //     .expect("tasks are serializable so it should not panic")
    //     .into_response()
}
async fn view_task(
    headers: HeaderMap,
    State(state): State<AppState>,
    Path(uuid): Path<Uuid>,
) -> impl IntoResponse {
    let authorized_status = is_authorized(&headers, &state, ClientPrivilege::View(uuid));
    if authorized_status != StatusCode::OK {
        return authorized_status.into_response();
    }
    if let Some(task) = state.tasks.lock().unwrap().iter().find(|t| t.id == uuid) {
        // bincode::serialize(&task)
        //     .expect("tasks are serializable so it should not panic")
        //     .into_response()
        bincode::encode_to_vec(task, state.config_bincode)
            .unwrap()
            .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}

async fn create_task(
    headers: HeaderMap,
    State(state): State<AppState>,
    body: Bytes,
) -> impl IntoResponse {
    // verify authorization
    let authorized_status = is_authorized(&headers, &state, ClientPrivilege::Creation);
    if authorized_status != StatusCode::OK {
        return authorized_status.into_response();
    }
    // get the body into a task
    if let Ok((new_task, _)) =
        bincode::decode_from_slice::<NewTask, Configuration>(&body, state.config_bincode)
    {
        let task = new_task.to_task();
        let uuid = task.id;
        let view_key = task.tokens.0.clone();
        let abort_key = task.tokens.1.clone();
        let update_key = task.tokens.2.clone();
        let endpoint = format!("/tasks/{}", uuid);
        // add task to the tasks in memory
        state.tasks.lock().unwrap().push(task);
        dbg!(&view_key);
        (
            StatusCode::CREATED,
            AppendHeaders([
                ("Content-Location", &endpoint),
                ("ViewToken", &view_key),
                ("AbortToken", &abort_key),
                ("UpdateToken", &update_key),
            ]),
        )
            .into_response()
    } else {
        // body was malformed
        StatusCode::BAD_REQUEST.into_response()
    }
}

// this endpoint can be used to update the progress or update the status to done or aborted.
// we know if it's using token for deleting or updating
async fn update_task(
    method: Method,
    headers: HeaderMap,
    Path(uuid): Path<Uuid>,
    State(state): State<AppState>,
    body: Bytes,
) -> impl IntoResponse {
    // verify authorization
    let authorized_status = if method == Method::PATCH {
        is_authorized(&headers, &state, ClientPrivilege::Update(uuid))
    } else {
        is_authorized(&headers, &state, ClientPrivilege::Abort(uuid))
    };
    if authorized_status != StatusCode::OK {
        return authorized_status.into_response();
    }
    let using_delete = method == Method::DELETE;
    // get the current task.
    let state_to_modify = state.tasks.clone();
    if let Some(current_task) = state
        .tasks
        .lock()
        .unwrap()
        .iter_mut()
        .find(|t| t.id == uuid)
    {
        // update only if status is currently active. Finished tasks must not be updated.
        // return a header with allowed method for this endpoint.
        if current_task.status != TaskStatus::Active {
            return (
                StatusCode::METHOD_NOT_ALLOWED,
                AppendHeaders([(ALLOW, "GET")]),
            )
                .into_response();
        }
        // only one of progress or status can be updated at once.
        // if status is different, progress is ignored.
        // else, only progress is updated.
        if let Ok(((progress, status, desc_finished, payload_finished), _)) =
            bincode::decode_from_slice::<(u8, TaskStatus, Option<String>, Vec<u8>), Configuration>(
                &body,
                state.config_bincode,
            )
        {
            if status == TaskStatus::Done && using_delete {
                return StatusCode::UNAUTHORIZED.into_response();
            }
            match status {
                TaskStatus::Done | TaskStatus::Aborted => {
                    // need to update task with new status
                    if let Some(desc_finished) = desc_finished {
                        current_task.description_result = desc_finished;
                    }
                    if !payload_finished.is_empty() {
                        current_task.payload_result = payload_finished;
                    }
                    current_task.status = status;
                    // need to send a request informing that the task is done for each push address.
                    let client = reqwest::Client::new();
                    for adr in current_task.push_address.iter() {
                        let adr = adr.clone();
                        let client = client.clone();
                        spawn(async move {
                            let _ = client.get(adr).send().await;
                        });
                    }
                    // need to start a timer before retiring the task
                    let seconds = current_task.duration;
                    spawn(async move {
                        sleep(Duration::from_secs(seconds.into())).await;
                        state_to_modify.lock().unwrap().retain(|t| t.id != uuid)
                    });
                }
                // if Status is Active, the progress must have been updated.
                _ => {
                    if using_delete {
                        return StatusCode::UNAUTHORIZED.into_response();
                    }
                    current_task.progress = progress;
                }
            };
            StatusCode::ACCEPTED.into_response()
        } else {
            StatusCode::BAD_REQUEST.into_response()
        }
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
