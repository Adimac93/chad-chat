use anyhow::Context;
use axum::{response::IntoResponse, http::StatusCode, Json};
use serde_json::json;
use sqlx::{Postgres, pool::PoolConnection, query};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Missing one or more fields")]
    MissingField,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            AppError::MissingField => StatusCode::BAD_REQUEST,
            AppError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            },
        };
        
        let info = match self {
            AppError::Unexpected(_) => "Unexpected server error".into(),
            _ => format!("{self:?}")
        };

        (status, Json(json!({ "error_info": info }))).into_response()
    }
}

pub async fn create_group(mut conn: PoolConnection<Postgres>, name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::MissingField);
    }

    query!(
        "
            insert into groups (name)
            values ($1)
        ",
        name
    )
    .execute(&mut conn)
    .await
    .context("Failed to create a group")?;

    Ok(())
}