use crate::utils::friends::errors::FriendError;
use crate::{
    utils::{
        auth::errors::AuthError, chat::errors::ChatError, groups::errors::GroupError,
        invitations::errors::InvitationError, roles::errors::RoleError
    },
};
use axum::response::IntoResponse;
use thiserror::Error;

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
}

// TODO: server error backtrace
impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        match self {
            AppError::AuthError(e) => return e.into_response(),
            AppError::GroupError(e) => return e.into_response(),
            AppError::ChatError(e) => return e.into_response(),
            AppError::FriendError(e) => return e.into_response(),
        };
    }
}

// better error conversion trait may be needed
impl From<InvitationError> for AppError {
    fn from(e: InvitationError) -> Self {
        AppError::from(GroupError::from(e))
    }
}

impl From<RoleError> for AppError {
    fn from(e: RoleError) -> Self {
        AppError::from(GroupError::from(e))
    }
}