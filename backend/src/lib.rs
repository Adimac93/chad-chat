pub mod configuration;
pub mod database;
pub mod models;
pub mod routes;
mod utils;

use axum::{
    extract::Path,
    http::header::CONTENT_TYPE,
    http::HeaderValue,
    response::{Html, IntoResponse},
    routing::get,
    Extension, Json, Router,
};
use configuration::get_config;
use serde_json::json;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;
use tracing::info;

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
        .route("/", get(home_page))
        .route("/:slug", get(not_found).post(not_found))
        .route("/health", get(health_check))
        .nest("/auth", routes::auth::router())
        .nest("/api", api)
        .nest("/chat", routes::chat::router())
        .layer(Extension(pool))
        .layer(cors)
}

async fn home_page() -> impl IntoResponse {
    // TODO: api docs, info
    Json(json!({"info":"docs"}))
}

async fn not_found(Path(slug): Path<String>) -> impl IntoResponse {
    let message = format!("endpoint '{slug}' isn't used");
    Json(json!({ "info": message }))
}

async fn health_check() -> impl IntoResponse {
    // TODO: database check and optional 3rd party server checks
    Json(json!({"info": "working"}))
}
