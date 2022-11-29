use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid username")]
    InvalidUsername(#[from] validator::ValidationErrors),
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
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            AuthError::InvalidUsername(_) => StatusCode::BAD_REQUEST,
            AuthError::UserAlreadyExists => StatusCode::BAD_REQUEST,
            AuthError::MissingCredential => StatusCode::BAD_REQUEST,
            AuthError::WeakPassword => StatusCode::BAD_REQUEST,
            AuthError::WrongUserOrPassword => StatusCode::UNAUTHORIZED,
            AuthError::InvalidToken => StatusCode::UNAUTHORIZED,
            AuthError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            AuthError::Unexpected(_) => "Unexpected server error".into(),
            AuthError::InvalidUsername(_) => "Invalid username".into(),
            _ => format!("{self:?}"),
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}
