pub mod app_errors;
pub mod configuration;
pub mod modules;
pub mod routes;
pub mod utils;

use axum::{
    http::header::CONTENT_TYPE,
    http::{HeaderValue, Method, StatusCode, Uri},
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};
use axum_extra::routing::SpaRouter;
use configuration::Settings;
use modules::{
    database::{get_postgres_pool, get_redis_pool, PgPool},
    extractors::{
        jwt::{JwtAccessSecret, JwtRefreshSecret, TokenExtractors},
        user_agent::UserAgentData,
    },
    smtp::Mailer,
};
use modules::{external_api::HttpClient, extractors::geolocation::NetworkData};
use serde_json::json;
use std::io;
use tower_http::cors::CorsLayer;
use tracing::{debug, error};

pub async fn app(config: Settings, test_pool: Option<PgPool>) -> Router {
    let pgpool = test_pool.unwrap_or(get_postgres_pool(config.postgres).await);
    let rdpool = get_redis_pool(config.redis).await;

    let http_client = HttpClient::new();

    let origin = config
        .app
        .origin
        .parse::<HeaderValue>()
        .expect("Invalid origin");
    let cors = CorsLayer::new()
        .allow_origin(origin)
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(true);

    let mailer = Mailer::new(config.smtp, config.app.origin);

    let groups = Router::new().nest(
        "/groups",
        routes::groups::router().nest("/invitations", routes::invitations::router()),
    );

    let spa = SpaRouter::new("/assets", "../frontend/dist/assets")
        .index_file("../index.html")
        .handle_error(not_found);

    let test = Router::new()
        .route("/geo", get(geolocation_info))
        .route("/ua", get(user_agent_info));

    let api = Router::new()
        .nest("/auth", routes::auth::router())
        .nest("/chat", routes::chat::router())
        .route("/health", get(health_check))
        .nest("/test", test)
        .merge(groups)
        .layer(Extension(pgpool))
        .layer(Extension(rdpool))
        .layer(Extension(http_client))
        .layer(Extension(mailer))
        .layer(Extension(TokenExtractors {
            access: JwtAccessSecret(config.app.access_jwt_secret),
            refresh: JwtRefreshSecret(config.app.refresh_jwt_secret),
        }))
        .layer(cors);

    Router::new().nest("/api", api).merge(spa)
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

async fn not_found(method: Method, uri: Uri, err: io::Error) -> String {
    let msg = format!("Method {method} for route {uri} caused error {err}");
    debug!("{msg}");
    msg
}

async fn geolocation_info(net: NetworkData) -> impl IntoResponse {
    debug!("Connection from {:?}", net.ip.ip().to_string());
    Json(json!({"ip": net.ip.ip(), "geo": net.geolocation_data}))
}
async fn user_agent_info(ua: UserAgentData) -> impl IntoResponse {
    debug!("User agent {ua:#?}");
    Json(ua)
}
