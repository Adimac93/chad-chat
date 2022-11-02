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
        .layer(Extension(queries::get_databse_pool().await))
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}
