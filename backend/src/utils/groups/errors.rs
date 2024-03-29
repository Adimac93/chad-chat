use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

use crate::utils::{invitations::errors::InvitationError, roles::errors::RoleError};

#[derive(Error, Debug)]
pub enum GroupError {
    #[error("User does not exist")]
    UserDoesNotExist,
    #[error("Group does not exist")]
    GroupDoesNotExist,
    #[error("User not in group")]
    UserNotInGroup,
    #[error("Missing one or more group fields")]
    MissingGroupField,
    #[error("Already in group")]
    UserAlreadyInGroup,
    #[error("Wrong invitation url")]
    BadInvitation,
    #[error("Invitation error")]
    InvitationError(#[from] InvitationError),
    #[error("Role error")]
    RoleError(#[from] RoleError),
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for GroupError {
    fn into_response(self) -> axum::response::Response {
        let info = match &self {
            GroupError::Unexpected(_) => "Unexpected server error".into(),
            _ => (&self).to_string(),
        };

        let status_code = match self {
            GroupError::UserDoesNotExist => StatusCode::BAD_REQUEST,
            GroupError::GroupDoesNotExist => StatusCode::BAD_REQUEST,
            GroupError::UserNotInGroup => StatusCode::FORBIDDEN,
            GroupError::MissingGroupField => StatusCode::BAD_REQUEST,
            GroupError::UserAlreadyInGroup => StatusCode::BAD_REQUEST,
            GroupError::BadInvitation => StatusCode::BAD_REQUEST,
            GroupError::InvitationError(e) => return e.into_response(),
            GroupError::RoleError(e) => return e.into_response(),
            GroupError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}

impl From<sqlx::Error> for GroupError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unexpected(anyhow::Error::from(e))
    }
}

impl From<redis::RedisError> for GroupError {
    fn from(e: redis::RedisError) -> Self {
        Self::Unexpected(anyhow::Error::from(e))
    }
}
