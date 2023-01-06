use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use thiserror::Error;

use crate::utils::groups::errors::GroupError;

#[derive(Error, Debug)]
pub enum InvitationError {
    #[error("Invitation is expired")]
    InvitationExpired,
    #[error("Unsupported invitation variant")]
    UnsupportedVariant,
    #[error("Invalid group invitation code")]
    InvalidCode,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for InvitationError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            InvitationError::InvitationExpired => StatusCode::BAD_REQUEST,
            InvitationError::UnsupportedVariant => StatusCode::BAD_REQUEST,
            InvitationError::InvalidCode => StatusCode::BAD_REQUEST,
            InvitationError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            InvitationError::Unexpected(_) => "Unexpected server error".into(),
            _ => self.to_string(),
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}

impl From<sqlx::Error> for InvitationError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unexpected(anyhow::Error::from(e))
    }
}
