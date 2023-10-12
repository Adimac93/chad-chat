pub mod configuration;
pub mod errors;
pub mod modules;
pub mod routes;
pub mod utils;

use axum::{
    extract::{FromRef, State},
    http::header::CONTENT_TYPE,
    http::{HeaderValue, Method, StatusCode, Uri},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use configuration::{ApplicationSettings, PostgresSettings, Settings, SmtpSettings};
use modules::{
    database::{get_postgres_pool, PgPool},
    extractors::{
        jwt::{JwtAccessSecret, JwtRefreshSecret, TokenExtractors},
        user_agent::UserAgentData,
    },
    smtp::Mailer,
};
use modules::{external_api::HttpClient, extractors::geolocation::NetworkData};
use serde_json::json;
use std::io;
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};
use utils::{
    chat::socket::ChatState,
    roles::models::{is_id_the_same, Gate, Role},
};
use uuid::Uuid;

#[macro_use]
pub extern crate tracing;

#[derive(FromRef, Clone)]
pub struct AppState {
    pub postgres: PgPool,
    pub client: HttpClient,
    pub smtp: Mailer,
    pub kick_gate: Gate<Role, (Uuid, Uuid)>,
    pub token_ext: TokenExtractors,
    pub chat_state: ChatState,
}

impl AppState {
    pub async fn new(config: Settings, test_pool: Option<PgPool>) -> Self {
        let kick_gate = Gate::build()
            .role(Role::Owner, 3)
            .role(Role::Admin, 1)
            .role(Role::Member, 0)
            .req(Role::Owner, 4)
            .req(Role::Admin, 1)
            .req(Role::Member, 0)
            .condition(is_id_the_same)
            .finish();

        let token_ext = TokenExtractors {
            access: JwtAccessSecret(config.app.access_jwt_secret),
            refresh: JwtRefreshSecret(config.app.refresh_jwt_secret),
        };

        let chat_state = ChatState::new();

        AppState {
            postgres: test_pool.unwrap_or(get_postgres_pool(config.postgres).await),
            client: HttpClient::new(),
            smtp: Mailer::new(config.smtp, config.app.origin),
            kick_gate,
            token_ext,
            chat_state,
        }
    }
}

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
        routes::groups::router().nest("/invitations", routes::invitations::router()),
    );

    let test = Router::new()
        .route("/geo", get(geolocation_info))
        .route("/ua", get(user_agent_info));

    let api = Router::new()
        .nest("/auth", routes::auth::router())
        .nest("/chat", routes::chat::router())
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

async fn health_check(State(pool): State<PgPool>) -> impl IntoResponse {
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
