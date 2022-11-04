pub mod routes;
pub mod auth;
pub mod models;
pub mod database;

use axum::{
    response::Html,
    routing::{get, post},
    Extension, Router,
};

pub async fn app() -> Router {
    Router::new()
        .route("/", get(handler))
        .route("/test", post(routes::auth::post_register_user))
        .route("/login-test", post(routes::auth::post_login_user))
        .route("/user-validation", post(routes::auth::protected_zone))
        .layer(Extension(database::get_database_pool().await))
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
