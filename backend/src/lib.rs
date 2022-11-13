pub mod configuration;
pub mod database;
pub mod models;
pub mod routes;
mod utils;

use axum::{http::header::CONTENT_TYPE, http::HeaderValue, Extension, Router};
use configuration::get_config;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

pub async fn app(pool: PgPool) -> Router {
    let config = get_config().expect("Failed to read configuration");

    let cors = CorsLayer::new()
        .allow_origin(
            config
                .origin
                .get()
                .parse::<HeaderValue>()
                .expect("Invalid origin"),
        )
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(true);

    let api = Router::new().nest("/groups", routes::groups::router());

    Router::new()
        .nest("/auth", routes::auth::router())
        .nest("/api", api)
        .nest("/chat", routes::chat::router())
        .layer(Extension(pool))
        .layer(cors)
}
