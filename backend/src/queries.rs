use anyhow::Context;
use axum::{response::IntoResponse, http::StatusCode, Json};
use serde_json::json;
use sqlx::{Postgres, pool::PoolConnection, query};
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Name is empty")]
    MissingField,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let (status, info) = match self {
            AppError::MissingField => (StatusCode::BAD_REQUEST, "Missing one or more fields"),
            AppError::Unexpected(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error"),
        };
        
        (status, Json(json!({ "error_info": info }))).into_response()
    }
}

pub async fn create_group(mut conn: PoolConnection<Postgres>, name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::MissingField);
    }

    let res = query!("
        insert into groups (name)
        values ($1)
    ", name)
    .execute(&mut conn)
    .await
    .context("Query failed")?;

    info!("{res:?}");
    Ok(())
}