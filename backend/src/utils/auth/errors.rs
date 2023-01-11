use axum::{http::StatusCode, response::IntoResponse, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Invalid username")]
    InvalidEmail(#[from] validator::ValidationErrors),
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Missing credential")]
    MissingCredential,
    #[error("Password is too weak")]
    WeakPassword,
    #[error("Incorrect email or password")]
    WrongEmailOrPassword,
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error("Maximum number of tags for this username")]
    TagOverflow,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            AuthError::InvalidEmail(_) => StatusCode::BAD_REQUEST,
            AuthError::UserAlreadyExists => StatusCode::BAD_REQUEST,
            AuthError::MissingCredential => StatusCode::BAD_REQUEST,
            AuthError::WeakPassword => StatusCode::BAD_REQUEST,
            AuthError::WrongEmailOrPassword => StatusCode::UNAUTHORIZED,
            AuthError::TagOverflow => StatusCode::BAD_REQUEST,
            AuthError::InvalidToken => StatusCode::UNAUTHORIZED,
            AuthError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            }
        };

        let info = match self {
            AuthError::Unexpected(_) => "Unexpected server error".to_string(),
            AuthError::InvalidEmail(e) => e.to_string(),
            _ => self.to_string(),
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}

impl From<sqlx::Error> for AuthError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unexpected(anyhow::Error::from(e))
    }
}
