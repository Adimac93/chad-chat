pub mod routes;
pub mod auth;
pub mod models;
pub mod database;
pub mod queries;

use axum::{
    response::Html,
    routing::{get, post},
    Extension, Router,
};

pub async fn app() -> Router {
    let auth_routes = Router::new()
        .route("/", get(handler))
        .route("/register", post(routes::auth::post_register_user))
        .route("/login", post(routes::auth::post_login_user))
        .route("/user-validation", post(routes::auth::protected_zone));
    let group_routes = Router::new()
        .route("/groups", post(routes::groups::post_create_group));
    
    Router::new()
        .nest("/", auth_routes)
        .nest("/", group_routes)
        .layer(Extension(database::get_database_pool().await))
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
