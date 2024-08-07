use error::TaskClientError;
use reqwest::{
    header::{HeaderValue, AUTHORIZATION, CONTENT_LOCATION},
    Client, Response,
};
/// re-export for client app
pub use tasks_tracker_common::{NewTask, Task, TaskStatus, BINCODE_CONFIG};
use url::Url;
pub mod error;
pub struct ResponseNewTask {
    pub location: Url,
    pub view_token: String,
    pub update_token: String,
    pub abort_token: String,
}

pub struct ErrorNewTask {}

pub async fn create_task(
    client: &Client,
    url_tt_api: &Url,
    new_task: &NewTask,
    token: &str,
) -> Result<ResponseNewTask, TaskClientError> {
    let body = bincode::encode_to_vec(new_task, BINCODE_CONFIG)?;
    let rep = client
        .post(url_tt_api.as_str())
        .header(
            AUTHORIZATION,
            HeaderValue::from_str(&["Bearer ", token].concat())?,
        )
        .body(body)
        .send()
        .await?
        .error_for_status()?;
    Ok(ResponseNewTask {
        location: rep_header_string(&rep, CONTENT_LOCATION.as_str())?.parse()?,
        view_token: rep_header_string(&rep, "ViewToken")?,
        abort_token: rep_header_string(&rep, "AbortToken")?,
        update_token: rep_header_string(&rep, "UpdateToken")?,
    })
}
pub async fn create_simple_task(
    client: &Client,
    url_tt_api: &Url,
    task_scope: String,
    task_name: String,
    token: &str,
) -> Result<ResponseNewTask, TaskClientError> {
    let body = bincode::encode_to_vec(
        NewTask {
            duration: 3600,
            scope: task_scope,
            name: task_name,
            description: String::new(),
            push_address: Vec::new(),
            payload: Vec::new(),
        },
        BINCODE_CONFIG,
    )?;
    let rep = client
        .post(url_tt_api.as_str())
        .header(
            AUTHORIZATION,
            HeaderValue::from_str(&["Bearer ", token].concat())?,
        )
        .body(body)
        .send()
        .await?
        .error_for_status()?;
    Ok(ResponseNewTask {
        location: rep_header_string(&rep, CONTENT_LOCATION.as_str())?.parse()?,
        view_token: rep_header_string(&rep, "ViewToken")?,
        abort_token: rep_header_string(&rep, "AbortToken")?,
        update_token: rep_header_string(&rep, "UpdateToken")?,
    })
}
fn rep_header_string(rep: &Response, key: &str) -> Result<String, TaskClientError> {
    Ok(rep
        .headers()
        .get(key)
        .ok_or(TaskClientError::HeaderNotFound(key.to_string()))?
        .to_str()?
        .to_string())
}
pub async fn update_task_progress(
    client: &Client,
    task_location: &Url,
    token: &str,
    new_progress: u8,
) -> Result<(), TaskClientError> {
    let body = bincode::encode_to_vec((new_progress, TaskStatus::Active), BINCODE_CONFIG)?;
    client
        .post(task_location.as_str())
        .header(
            AUTHORIZATION,
            HeaderValue::from_str(&["Bearer ", token].concat())?,
        )
        .body(body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
pub async fn finish_task(
    client: &Client,
    task_location: &Url,
    token: &str,
    description_result: Option<&str>,
    payload_result: &[u8],
) -> Result<(), TaskClientError> {
    let body = bincode::encode_to_vec(
        (100u8, TaskStatus::Done, description_result, payload_result),
        BINCODE_CONFIG,
    )?;
    client
        .post(task_location.as_str())
        .header(
            AUTHORIZATION,
            HeaderValue::from_str(&["Bearer ", token].concat())?,
        )
        .body(body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
pub async fn abort_task(
    client: &Client,
    task_location: &Url,
    token: &str,
    description_result: Option<&str>,
    payload_result: &[u8],
) -> Result<(), TaskClientError> {
    let body = bincode::encode_to_vec(
        (0u8, TaskStatus::Aborted, description_result, payload_result),
        BINCODE_CONFIG,
    )?;
    client
        .post(task_location.as_str())
        .header(
            AUTHORIZATION,
            HeaderValue::from_str(&["Bearer ", token].concat())?,
        )
        .body(body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
pub async fn get_task(
    client: &Client,
    token: &str,
    task_location: &Url,
) -> Result<Task, TaskClientError> {
    Ok(bincode::decode_from_slice(
        &client
            .get(task_location.as_str())
            .header(
                AUTHORIZATION,
                HeaderValue::from_str(&["Bearer ", token].concat())?,
            )
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?,
        BINCODE_CONFIG,
    )?
    .0)
}
