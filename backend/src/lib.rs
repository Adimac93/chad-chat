pub mod auth;
pub mod database;
pub mod models;
pub mod queries;
pub mod routes;
pub mod groups;

use std::sync::{Arc};

use axum::{
    routing::{get, post},
    Extension, Router, http::HeaderValue,
    http::header::{CONTENT_TYPE,}
};
use routes::{chat::{chat_index, AppState, chat_handler, get_user_groups}, auth::{login_index, register_index}, groups::{get_join_group_by_link, InvitationState, post_create_group_invitation_link}};
use sqlx::PgPool;
use tower_http::cors::{CorsLayer};

pub async fn app(pool: PgPool) -> Router {
    let cors = CorsLayer::new().allow_origin("http://localhost:5173".parse::<HeaderValue>().unwrap()).allow_headers([CONTENT_TYPE]).allow_credentials(true); // allow_credentials "http://localhost:3000".parse::<HeaderValue>().unwrap()

    let auth_routes = Router::new()
        .route("/register", get(register_index).post(routes::auth::post_register_user))
        .route("/login", get(login_index).post(routes::auth::post_login_user))
        .route("/user-validation", post(routes::auth::protected_zone));

    let group_routes = Router::new()
        .route("/groups", post(routes::groups::post_create_group))
        .route("/add-user", post(routes::groups::post_add_user_to_group))
        .route("/groups/create",post(post_create_group_invitation_link))
        .route("/groups/join/:invite_id",get(get_join_group_by_link))
        .layer(Extension(Arc::new(InvitationState::new())));

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
