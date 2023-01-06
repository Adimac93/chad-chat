use axum::response::IntoResponse;
use axum::{http::StatusCode, Json};
use serde_json::json;
use sqlx::{migrate, PgPool};
use thiserror::Error;
use tracing::error;

use crate::configuration::DatabaseSettings;

pub async fn get_database_pool(config: DatabaseSettings) -> PgPool {
    let pool = PgPool::connect(&config.get_connection_string())
        .await
        .expect("Cannot establish database connection");
    if config.is_migrating() {
        migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Auto migration failed");
    }
    pool
}

#[derive(Error, Debug)]
#[error(transparent)]
pub struct DatabaseError(#[from] pub sqlx::Error);

impl IntoResponse for DatabaseError {
    fn into_response(self) -> axum::response::Response {
        error!("Database error: {:#?}", self.0);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error_info": "Unexpected error"})),
        )
            .into_response()
    }
}
