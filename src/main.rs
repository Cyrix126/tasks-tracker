use bincode::config::Configuration;
use bincode::Decode;
use bincode::Encode;
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

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
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use tokio::{spawn, time::sleep};
use url::Url;
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

// Possible status variant of a task.
#[derive(Clone, Default, PartialEq, Encode, Decode)]
enum TaskStatus {
    // the task has been started and is currently progressing.
    #[default]
    Active,
    // the task has been aborted.
    Aborted,
    // the task finished successfully.
    Done,
}

#[derive(Clone, Encode)]
#[cfg_attr(test, derive(Decode))]
struct Task {
    // identifier of the task that will be provided when created.
    // #[serde(skip_deserializing)]
    #[bincode(with_serde)]
    id: Uuid,
    // timelapse after which the task will be discarded.
    duration: u32,
    // Name of service creating the task. Information given by client.
    scope: String,
    // Name of the task to identify it in a human readable way.
    name: String,
    // description of the task
    description: String,
    // Progress in % updated by client with progress/status write access. R
    progress: u8,
    status: TaskStatus,
    // Tokens to access the task.
    // The first is to view progress and status.
    // Second one is only to change the status to abort.
    // Third one is to update the progress and status.
    tokens: (String, String, String),
    // Url where to send push notifications.
    // #[bitcode(with_serde)]
    #[bincode(with_serde)]
    push_address: Vec<Url>,
}
// body that is sent when creating a new task.
#[derive(Decode)]
#[cfg_attr(test, derive(Encode))]
struct NewTask {
    duration: u32,
    scope: String,
    name: String,
    description: String,
    #[bincode(with_serde)]
    push_address: Vec<Url>,
}

impl NewTask {
    #[allow(clippy::wrong_self_convention)]
    fn to_task(self) -> Task {
        Task {
            id: Uuid::new_v4(),
            duration: self.duration,
            scope: self.scope,
            name: self.name,
            description: self.description,
            progress: 0,
            status: TaskStatus::Active,
            tokens: (
                thread_rng()
                    .sample_iter(Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect(),
                thread_rng()
                    .sample_iter(Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect(),
                thread_rng()
                    .sample_iter(Alphanumeric)
                    .take(32)
                    .map(char::from)
                    .collect(),
            ),
            push_address: self.push_address,
        }
    }
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
                ("Location", &endpoint),
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
        if let Ok(((progress, status), _)) = bincode::decode_from_slice::<
            (u8, TaskStatus),
            Configuration,
        >(&body, state.config_bincode)
        {
            if status == TaskStatus::Done && using_delete {
                return StatusCode::UNAUTHORIZED.into_response();
            }
            match status {
                TaskStatus::Done | TaskStatus::Aborted => {
                    // need to update task with new status
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
