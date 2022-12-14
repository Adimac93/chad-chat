use axum::response::IntoResponse;
use thiserror::Error;
use crate::utils::{
    auth::errors::AuthError,
    groups::errors::GroupError,
    chat::errors::ChatError, invitations::errors::InvitationError
};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Auth error")]
    AuthError(#[from] AuthError),
    #[error("Group error")]
    GroupError(#[from] GroupError),
    #[error("Chat error")]
    ChatError(#[from] ChatError),
    // #[error(transparent)]
    // Unexpected(#[from] anyhow::Error),
}

// TODO: server error backtrace
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::AuthError(e) => return e.into_response(),
            AppError::GroupError(e) => return e.into_response(),
            AppError::ChatError(e) => return e.into_response(),
            // AppError::Unexpected(e) => todo!(),
        };
    }
}

// better error conversion trait may be needed
impl From<InvitationError> for AppError {
    fn from(e: InvitationError) -> Self {
        AppError::from(GroupError::from(e))
    }
}