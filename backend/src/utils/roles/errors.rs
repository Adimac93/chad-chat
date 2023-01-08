use axum::{response::IntoResponse, Json, http::StatusCode};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RoleError {
    #[error("Role change rejected")]
    RoleChangeRejection,
    #[error("Invalid role name")]
    RoleParseError,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for RoleError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            RoleError::RoleChangeRejection => StatusCode::BAD_REQUEST,
            RoleError::RoleParseError => StatusCode::BAD_REQUEST,
            RoleError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            RoleError::Unexpected(_) => "Unexpected server error".into(),
            _ => self.to_string(),
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}

impl From<sqlx::Error> for RoleError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unexpected(anyhow::Error::from(e))
    }
}

impl From<serde_json::Error> for RoleError {
    fn from(e: serde_json::Error) -> Self {
        Self::Unexpected(anyhow::Error::from(e))
    }
}
