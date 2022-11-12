use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum GroupError {
    #[error("User not in group")]
    UserNotInGroup,
    #[error("Missing one or more group fields")]
    MissingGroupField,
    #[error("Already in group")]
    UserAlreadyInGroup,
    #[error("Wrong invitation url")]
    BadInvitation,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for GroupError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            GroupError::UserNotInGroup => StatusCode::FORBIDDEN,
            GroupError::MissingGroupField => StatusCode::BAD_REQUEST,
            GroupError::UserAlreadyInGroup => StatusCode::BAD_REQUEST,
            GroupError::BadInvitation => StatusCode::BAD_REQUEST,
            GroupError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            GroupError::Unexpected(_) => "Unexpected server error".into(),
            _ => format!("{self:?}"),
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}
