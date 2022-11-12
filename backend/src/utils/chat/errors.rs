use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ChatError {
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for ChatError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            ChatError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            ChatError::Unexpected(_) => "Unexpected server error".into(),
            _ => format!("{self:?}"),
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}
