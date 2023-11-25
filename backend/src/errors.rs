use anyhow::anyhow;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use sqlx::error::ErrorKind;
use thiserror::Error;
use tracing::{debug, error};
use typeshare::typeshare;

const BACKTRACE_DEPTH: usize = 5;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("{code} - {message}")]
    Expected { code: StatusCode, message: String },
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

#[typeshare]
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

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let error_message = self.to_string();
        let (code, message) = match self {
            AppError::Expected { code, message } => {
                debug!("{error_message}");
                (code, ErrorResponse::json(message))
            }
            AppError::Unexpected(e) => {
                let backtrace = e.backtrace();
                let filtered_backtrace = backtrace.to_string().lines().take(2*BACKTRACE_DEPTH).collect::<Vec<&str>>().join("\n");
                if &filtered_backtrace == "disabled backtrace" {
                    error!("{error_message}");
                } else {
                    error!("{error_message}\n\n{filtered_backtrace}");
                }

                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    ErrorResponse::json("Unexpected server error".into()),
                )
            }
        };
        (code, message).into_response()
    }
}

impl AppError {
    pub fn exp(code: StatusCode, message: &str) -> Self {
        Self::Expected {
            code,
            message: message.to_string(),
        }
    }
}

impl From<sqlx::Error> for AppError {
    fn from(val: sqlx::Error) -> Self {
        Self::Unexpected(anyhow!(val))
    }
}

pub struct DbErrMessage {
    pub err: AppError,
    pub kind: ErrorKind,
    pub name: String,
}

impl DbErrMessage {
    pub fn new(err: sqlx::Error) -> Self {
        let (kind, name) = if let Some(e) = err.as_database_error() {
            (e.kind(), e.constraint().unwrap_or_default().to_string())
        } else {
            (ErrorKind::Other, "".to_string())
        };

        Self {
            err: AppError::from(err),
            kind,
            name,
        }
    }

    pub fn fk(mut self, code: StatusCode, message: &str) -> Self {
        if self.kind == ErrorKind::ForeignKeyViolation {
            self.err = AppError::exp(code, message);
        }
        self
    }

    pub fn unique(mut self, code: StatusCode, message: &str) -> Self {
        if self.kind == ErrorKind::UniqueViolation {
            self.err = AppError::exp(code, message);
        }
        self
    }

    pub fn not_null(mut self, code: StatusCode, message: &str) -> Self {
        if self.kind == ErrorKind::NotNullViolation {
            self.err = AppError::exp(code, message);
        }
        self
    }

    pub fn check(mut self, code: StatusCode, message: &str) -> Self {
        if self.kind == ErrorKind::CheckViolation {
            self.err = AppError::exp(code, message);
        }
        self
    }
}

impl From<DbErrMessage> for AppError {
    fn from(val: DbErrMessage) -> Self {
        val.err
    }
}
