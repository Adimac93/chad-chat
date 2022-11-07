pub mod auth;
pub mod database;
pub mod models;
pub mod queries;
pub mod routes;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Extension, Router,
};
use routes::chat::{chat, AppState, chat_handler};
use sqlx::PgPool;
use tower_http::cors::{Any, CorsLayer};

pub async fn app(pool: PgPool) -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_headers(Any);

    let auth_routes = Router::new()
        .route("/register", post(routes::auth::post_register_user))
        .route("/login", post(routes::auth::post_login_user))
        .route("/user-validation", post(routes::auth::protected_zone));

    let group_routes = Router::new().route("/groups", post(routes::groups::post_create_group));

    let socket_routes = Router::new()
        .route("/", get(chat))
        .route("/websocket",get(chat_handler))
        .layer(Extension(Arc::new(AppState::new())));

    Router::new()
        .nest("/auth", auth_routes)
        .nest("/api", group_routes)
        .nest("/chat", socket_routes)
        .layer(Extension(pool))
        .layer(cors)
}
