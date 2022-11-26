pub mod configuration;
pub mod database;
pub mod models;
pub mod routes;
mod utils;

use axum::{
    extract::Path,
    http::header::CONTENT_TYPE,
    http::HeaderValue,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Extension, Json, Router,
};
use configuration::get_config;
use secrecy::Secret;
use serde_json::json;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

pub async fn app(pool: PgPool) -> Router {
    let config = get_config().expect("Failed to read configuration");

    let origin = config
        .app
        .origin
        .parse::<HeaderValue>()
        .expect("Invalid origin");
    let cors = CorsLayer::new()
        .allow_origin(origin)
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
        .layer(Extension(JwtSecret(config.app.jwt_key)))
        .layer(cors)
}

#[derive(Clone)]
pub struct JwtSecret(pub Secret<String>);

async fn home_page() -> impl IntoResponse {
    // TODO: api docs, info
    Json(json!({"info":"docs"}))
}

async fn not_found(Path(slug): Path<String>) -> impl IntoResponse {
    let message = format!("endpoint '{slug}' isn't used");
    (StatusCode::NOT_FOUND, Json(json!({ "info": message })))
}

async fn health_check(Extension(pool): Extension<PgPool>) -> impl IntoResponse {
    let is_database_connected = sqlx::query("select 1").fetch_one(&pool).await.is_ok();
    if is_database_connected {
        return (
            StatusCode::OK,
            Json(json!({"status": "all backend services are working properly"})),
        );
    }
    (
        StatusCode::SERVICE_UNAVAILABLE,
        Json(json!({"status":"database unavailable"})),
    )
}
