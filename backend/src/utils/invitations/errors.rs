use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use thiserror::Error;

use crate::utils::roles::errors::RoleError;

#[derive(Error, Debug)]
pub enum NotAppError {
    #[error("Invitation is expired")]
    InvitationExpired,
    #[error("Unsupported invitation variant")]
    UnsupportedVariant,
    #[error("Invalid group invitation code")]
    InvalidCode,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for NotAppError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            AppError::InvitationExpired => StatusCode::BAD_REQUEST,
            AppError::UnsupportedVariant => StatusCode::BAD_REQUEST,
            AppError::InvalidCode => StatusCode::BAD_REQUEST,
            AppError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            AppError::Unexpected(_) => "Unexpected server error".into(),
            _ => self.to_string(),
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}
