pub mod configuration;
pub mod database;
pub mod models;
pub mod routes;
pub mod utils;

use std::io;

use axum::{
    http::header::CONTENT_TYPE,
    http::StatusCode,
    http::{HeaderValue, Method, Uri},
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};
use axum_extra::routing::SpaRouter;
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

    let groups = Router::new().nest(
        "/groups",
        routes::groups::router().nest("/invitations", routes::invitations::router()),
    );

    let spa = SpaRouter::new("/assets", "../frontend/dist/assets")
        .index_file("../index.html")
        .handle_error(not_found);

    let api = Router::new()
        .nest("/auth", routes::auth::router())
        .nest("/chat", routes::chat::router())
        .route("/health", get(health_check))
        .merge(groups)
        .layer(Extension(pool))
        .layer(Extension(JwtSecret(config.app.access_jwt_secret)))
        .layer(Extension(RefreshJwtSecret(config.app.refresh_jwt_secret)))
        // .route("/:slug", get(not_found).post(not_found))
        .layer(cors);

    Router::new().nest("/api", api).merge(spa)
}

#[derive(Clone)]
pub struct JwtSecret(pub Secret<String>);

#[derive(Clone)]
pub struct RefreshJwtSecret(pub Secret<String>);

async fn home_page() -> impl IntoResponse {
    // TODO: api docs, info
    Json(json!({"info":"docs"}))
}

async fn not_found(method: Method, uri: Uri, err: io::Error) -> String {
    format!("Method {method} for route {uri} caused error {err}")
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
