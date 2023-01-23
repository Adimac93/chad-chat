use crate::configuration::{ConnectionPrep, PostgresSettings, RedisSettings};
use axum::response::IntoResponse;
use axum::{http::StatusCode, Json};
use redis::aio::ConnectionManager;
use redis::Client;
use serde_json::json;
use sqlx::migrate;
pub use sqlx::PgPool;
use thiserror::Error;
use tracing::error;

/// An alias for [`ConnectionManager`][redis::aio::ConnectionManager], specialized for Redis.
pub type RdPool = ConnectionManager;

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

pub async fn get_redis_pool(config: RedisSettings) -> RdPool {
    let client =
        Client::open(config.get_connection_string()).expect("Cannot establish redis connection");
    client
        .get_tokio_connection_manager()
        .await
        .expect("Failed to get redis connection manager")
}

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Postgres error")]
    PostgresError(#[from] sqlx::Error),
    #[error("Redis error")]
    RedisError(#[from] redis::RedisError),
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
