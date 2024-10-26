use derive_more::derive::Deref;
use error::TaskClientError;
use reqwest::header::InvalidHeaderValue;
use reqwest::{header, RequestBuilder};
use reqwest::{
    header::{HeaderValue, AUTHORIZATION, CONTENT_LOCATION},
    Client as ReqClient, ClientBuilder, Response,
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

#[derive(Deref)]
pub struct Client {
    #[deref]
    client: ReqClient,
    default_url: Url,
}

impl Client {
    pub fn new(mut uri: Url) -> Result<Self, TaskClientError> {
        let mut headers = header::HeaderMap::new();
        let mut auth_value = header::HeaderValue::from_str(
            &["Bearer ", uri.password().unwrap_or_default()].concat(),
        )?;
        auth_value.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, auth_value);
        uri.set_password(None).unwrap();
        Ok(Client {
            client: ClientBuilder::new()
                .default_headers(headers)
                .build()
                .unwrap(),
            default_url: uri,
        })
    }
    pub async fn create_task(
        &self,
        new_task: NewTask,
        token: Option<&str>,
    ) -> Result<ResponseNewTask, TaskClientError> {
        let body = bincode::encode_to_vec(new_task, BINCODE_CONFIG)?;
        let rep = request_with_token(self.post(self.default_url.as_str()), token)?
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
        &self,
        task_scope: String,
        task_name: String,
        token: Option<&str>,
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
        let rep = request_with_token(self.post(self.default_url.as_str()), token)?
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
    pub async fn update_task_progress(
        &self,
        task_location: &Url,
        new_progress: u8,
        token: Option<&str>,
    ) -> Result<(), TaskClientError> {
        let body = bincode::encode_to_vec((new_progress, TaskStatus::Active), BINCODE_CONFIG)?;
        request_with_token(self.post(task_location.as_str()), token)?
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }
    pub async fn finish_task(
        &self,
        task_location: &Url,
        description_result: Option<&str>,
        payload_result: &[u8],
        token: Option<&str>,
    ) -> Result<(), TaskClientError> {
        let body = bincode::encode_to_vec(
            (100u8, TaskStatus::Done, description_result, payload_result),
            BINCODE_CONFIG,
        )?;
        request_with_token(self.post(task_location.as_str()), token)?
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        Ok::<(), TaskClientError>(())
    }
    pub async fn abort_task(
        &self,
        task_location: &Url,
        description_result: Option<&str>,
        payload_result: &[u8],
        token: Option<&str>,
    ) -> Result<(), TaskClientError> {
        let body = bincode::encode_to_vec(
            (0u8, TaskStatus::Aborted, description_result, payload_result),
            BINCODE_CONFIG,
        )?;
        request_with_token(self.post(task_location.as_str()), token)?
            .body(body)
            .send()
            .await?
            .error_for_status()?;
        Ok::<(), TaskClientError>(())
    }
    pub async fn get_task(
        &self,
        task_location: &Url,
        token: Option<&str>,
    ) -> Result<Task, TaskClientError> {
        Ok(bincode::decode_from_slice(
            &request_with_token(self.get(task_location.as_str()), token)?
                .send()
                .await?
                .error_for_status()?
                .bytes()
                .await?,
            BINCODE_CONFIG,
        )?
        .0)
    }
}
fn rep_header_string(rep: &Response, key: &str) -> Result<String, TaskClientError> {
    Ok(rep
        .headers()
        .get(key)
        .ok_or(TaskClientError::HeaderNotFound(key.to_string()))?
        .to_str()?
        .to_string())
}
fn request_with_token(
    req: RequestBuilder,
    token: Option<&str>,
) -> Result<RequestBuilder, InvalidHeaderValue> {
    if let Some(token) = token {
        return Ok(req.header(
            AUTHORIZATION,
            HeaderValue::from_str(&["Bearer ", token].concat())?,
        ));
    }
    Ok(req)
}
