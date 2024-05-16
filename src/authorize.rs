use axum::http::{header::AUTHORIZATION, HeaderMap};
use reqwest::StatusCode;
use uuid::Uuid;

use crate::{AppState, ClientPrivilege, Task};

/// functions for managing grant access to endpoints.
// will return ok if is authorized, forbidden if key exist but is not valid for this endpoint and unauthorized for inexistent key.
pub(crate) fn is_authorized(
    headers: &HeaderMap,
    state: &AppState,
    privilege_required: ClientPrivilege,
) -> StatusCode {
    if let Some(value) = headers.get(AUTHORIZATION) {
        if let Ok(token) = value.to_str() {
            if state
                .token_admin
                .as_ref()
                .is_some_and(|admin| token == ["Bearer ", admin].concat())
                || match privilege_required {
                    ClientPrivilege::Creation => Some(state.token_create.to_string()),
                    ClientPrivilege::View(uuid) => {
                        if let Some(task) = task_with_uuid(state, &uuid) {
                            Some(task.tokens.0)
                        } else {
                            None
                        }
                    }
                    ClientPrivilege::Abort(uuid) => {
                        if let Some(task) = task_with_uuid(state, &uuid) {
                            Some(task.tokens.1)
                        } else {
                            None
                        }
                    }
                    ClientPrivilege::Update(uuid) => {
                        if let Some(task) = task_with_uuid(state, &uuid) {
                            Some(task.tokens.2)
                        } else {
                            None
                        }
                    }
                    ClientPrivilege::List => state.token_admin.clone(),
                }
                .is_some_and(|valid_token| ["Bearer ", &valid_token].concat() == token)
            {
                return StatusCode::OK;
            } else if !state
                .tasks
                .lock()
                .unwrap()
                .iter()
                .filter(|t| {
                    ["Bearer ", &t.tokens.0].concat() == token
                        || ["Bearer ", &t.tokens.1].concat() == token
                        || ["Bearer ", &t.tokens.2].concat() == token
                        || ["Bearer ", &state.token_create].concat() == token
                })
                .collect::<Vec<&Task>>()
                .is_empty()
            {
                return StatusCode::FORBIDDEN;
            } else {
                return StatusCode::UNAUTHORIZED;
            }
        } else {
            return StatusCode::BAD_REQUEST;
        }
    }
    {
        StatusCode::UNAUTHORIZED
    }
}

// simple function to get the task by his uuid.
pub(crate) fn task_with_uuid(state: &AppState, uuid: &Uuid) -> Option<Task> {
    state
        .tasks
        .lock()
        .unwrap()
        .iter()
        .find(|t| &t.id == uuid)
        .cloned()
}
