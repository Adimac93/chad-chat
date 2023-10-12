use crate::configuration::{ConnectionPrep, PostgresSettings};
use axum::response::IntoResponse;
use axum::{http::StatusCode, Json};
use serde_json::json;
use sqlx::migrate;
pub use sqlx::PgPool;
use thiserror::Error;
use tracing::error;

pub async fn get_postgres_pool(config: PostgresSettings) -> PgPool {
    let pool = PgPool::connect(&config.get_connection_string())
        .await
        .expect("Cannot establish postgres connection");
    if config.is_migrating() {
        migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Auto migration failed");
    }
    pool
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Postgres error")]
    PostgresError(#[from] sqlx::Error),
}

impl IntoResponse for DatabaseError {
    fn into_response(self) -> axum::response::Response {
        error!("Database error: {:#?}", self);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error_info": "Unexpected error"})),
        )
            .into_response()
    }
}
