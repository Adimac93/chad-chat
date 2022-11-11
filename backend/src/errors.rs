use axum::{
    response::IntoResponse,
    http::StatusCode, Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Missing credential")]
    MissingCredential,
    #[error("Password is too weak")]
    WeakPassword,
    #[error("Incorrect user or password")]
    WrongUserOrPassword,
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error)
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            AuthError::UserAlreadyExists => StatusCode::BAD_REQUEST,
            AuthError::MissingCredential => StatusCode::BAD_REQUEST,
            AuthError::WeakPassword => StatusCode::BAD_REQUEST,
            AuthError::WrongUserOrPassword => StatusCode::UNAUTHORIZED,
            AuthError::InvalidToken => StatusCode::UNAUTHORIZED,
            AuthError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            },
        };
        
        let info = match self {
            AuthError::Unexpected(_) => "Unexpected server error".into(),
            _ => format!("{self:?}")
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}

#[derive(Error, Debug)]
pub enum GroupError {
    #[error("Missing one or more group fields")]
    MissingGroupField,
    #[error("Already in group")]
    UserAlreadyInGroup,
    #[error("Wrong invitation url")]
    BadInvitation,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error)
}

impl IntoResponse for GroupError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            GroupError::MissingGroupField => StatusCode::BAD_REQUEST,
            GroupError::UserAlreadyInGroup => StatusCode::BAD_REQUEST,
            GroupError::BadInvitation => StatusCode::BAD_REQUEST,
            GroupError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            },
        };

        let info = match self {
            GroupError::Unexpected(_) => "Unexpected server error".into(),
            _ => format!("{self:?}")
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}

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
