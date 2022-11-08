pub mod auth;
pub mod database;
pub mod models;
pub mod queries;
pub mod routes;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Extension, Router, http::HeaderValue,
};
use routes::{chat::{chat_index, AppState, chat_handler, get_user_groups}, auth::{login_index, register_index}};
use sqlx::PgPool;
use tower_http::cors::{CorsLayer};

pub async fn app(pool: PgPool) -> Router {
    let cors = CorsLayer::new().allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap()).allow_credentials(true); // allow_credentials "http://localhost:3000".parse::<HeaderValue>().unwrap()

    let auth_routes = Router::new()
        .route("/register", get(register_index).post(routes::auth::post_register_user))
        .route("/login", get(login_index).post(routes::auth::post_login_user))
        .route("/user-validation", post(routes::auth::protected_zone));

    let group_routes = Router::new().route("/groups", post(routes::groups::post_create_group));

    let socket_routes = Router::new()
        .route("/", get(chat_index))
        .route("/websocket",get(chat_handler))
        .route("/groups", get(get_user_groups))
        .layer(Extension(Arc::new(AppState::new())));

    Router::new()
        .nest("/auth", auth_routes)
        .nest("/api", group_routes)
        .nest("/chat", socket_routes)
        .layer(Extension(pool))
        .layer(cors)
}
