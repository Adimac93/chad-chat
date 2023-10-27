

use axum::{
    debug_handler, extract::State, http::HeaderValue, response::IntoResponse, routing::get, Json,
    Router,
};
use hyper::{header::CONTENT_TYPE, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

use crate::{
    configuration::Settings,
    modules::extractors::{geolocation::NetworkData, user_agent::UserAgentData},
    state::AppState,
};

pub mod auth;
pub mod chat;
pub mod friends;
pub mod groups;
pub mod invitations;

pub async fn app(config: Settings, test_pool: Option<PgPool>) -> Router {
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
        groups::router().nest("/invitations", invitations::router()),
    );

    let test = Router::new()
        .route("/geo", get(geolocation_info))
        .route("/ua", get(user_agent_info));

    let api = Router::new()
        .nest("/auth", auth::router())
        .nest("/chat", chat::router())
        .route("/health", get(health_check))
        .nest("/test", test)
        .merge(groups)
        .with_state(AppState::new(config, test_pool).await)
        .layer(cors);

    Router::new().nest("/api", api).nest_service(
        "/",
        ServeDir::new("../frontend/dist")
            .not_found_service(ServeFile::new("../frontend/dist/index.html")),
    )
}

#[debug_handler]
async fn health_check(State(pool): State<PgPool>) -> impl IntoResponse {
    let is_database_connected = sqlx::query("SELECT 1").fetch_one(&pool).await.is_ok();
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

#[debug_handler]
async fn geolocation_info(State(_): State<AppState>, net: NetworkData) -> impl IntoResponse {
    debug!("Connection from {:?}", net.ip.ip().to_string());
    Json(json!({"ip": net.ip.ip(), "geo": net.geolocation_data}))
}

#[debug_handler]
async fn user_agent_info(State(_): State<AppState>, ua: UserAgentData) -> impl IntoResponse {
    debug!("User agent {ua:#?}");
    Json(ua)
}
