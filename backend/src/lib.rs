mod queries;

use axum::{
    response::Html,
    routing::{get, post},
    Extension, Router,
};

pub async fn app() -> Router {
    Router::new()
        .route("/", get(handler))
        .route("/test", post(queries::post_register_user))
        .route("/login-test", post(queries::post_login_user))
        .route("/user-validation", post(queries::protected_zone))
        .layer(Extension(queries::get_database_pool().await))
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
