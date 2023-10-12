use crate::utils::friends::errors::FriendError;
use crate::utils::{
    auth::errors::AuthError, chat::errors::ChatError, groups::errors::GroupError,
    invitations::errors::InvitationError, roles::errors::RoleError,
};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use thiserror::Error;
use tracing::error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error(transparent)]
    AuthError(#[from] AuthError),
    #[error(transparent)]
    GroupError(#[from] GroupError),
    #[error(transparent)]
    ChatError(#[from] ChatError),
    #[error(transparent)]
    FriendError(#[from] FriendError),
    #[error(transparent)]
    Unexpected(anyhow::Error),
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ErrorResponse {
    error: String,
}

impl ErrorResponse {
    fn json(error: String) -> Json<Self> {
        Json(Self { error })
    }
}

// TODO: server error backtrace
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::AuthError(e) => return e.into_response(),
            AppError::GroupError(e) => return e.into_response(),
            AppError::ChatError(e) => return e.into_response(),
            AppError::FriendError(e) => return e.into_response(),
            AppError::Unexpected(_) => {
                let error_message = self.to_string();
                error!("{error_message}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse::json("Unexpected server error".into()),
                )
                    .into_response()
            }
        }
    }
}

// better error conversion trait may be needed
impl From<InvitationError> for AppError {
    fn from(e: InvitationError) -> Self {
        Self::from(GroupError::from(e))
    }
}

impl From<RoleError> for AppError {
    fn from(e: RoleError) -> Self {
        Self::from(GroupError::from(e))
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unexpected(anyhow::anyhow!(e))
    }
}
