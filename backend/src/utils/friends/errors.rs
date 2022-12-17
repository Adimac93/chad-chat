use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FriendError {
    #[error("Already a friend")]
    AlreadyFriend,
    #[error("Friend request already sent")]
    RequestSendAlready,
    #[error("Friend request is missing")]
    RequestMissing,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for FriendError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            FriendError::AlreadyFriend => StatusCode::BAD_REQUEST,
            FriendError::RequestSendAlready => StatusCode::BAD_REQUEST,
            FriendError::RequestMissing => StatusCode::BAD_REQUEST,
            FriendError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            FriendError::Unexpected(_) => "Unexpected server error".into(),
            _ => self.to_string(),
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}
