pub mod app_errors;
pub mod configuration;
pub mod database;
pub mod external_api;
pub mod routes;
pub mod utils;

use std::{io, net::IpAddr};

use axum::{
    async_trait,
    extract::{self, ConnectInfo, FromRequest, Host, RequestParts},
    http::header::CONTENT_TYPE,
    http::{self, StatusCode},
    http::{HeaderValue, Method, Uri},
    response::IntoResponse,
    routing::get,
    Extension, Json, Router,
};
use axum_extra::routing::SpaRouter;
use database::{get_postgres_pool, get_redis_pool};
use external_api::{GeolocationData, HttpClient, UserAgentData};
use configuration::{ApplicationSettings, Settings};
use secrecy::{ExposeSecret, Secret};
use serde::Serialize;
use serde_json::json;
use tower_http::cors::CorsLayer;
use tracing::{debug, error};
use utils::auth::errors::AuthError;

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
        .route("/ua", get(geolocation_info));

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
        .layer(Extension(TokenExtensions {
            access: JwtAccessSecret(config.app.access_jwt_secret),
            refresh: JwtRefreshSecret(config.app.refresh_jwt_secret),
        }))
        .layer(cors);

    Router::new().nest("/api", api).merge(spa)
}

#[derive(Clone)]
pub struct JwtAccessSecret(pub Secret<String>);

#[derive(Clone)]
pub struct JwtRefreshSecret(pub Secret<String>);

#[derive(Clone)]
pub struct TokenExtensions {
    access: JwtAccessSecret,
    refresh: JwtRefreshSecret,
}

#[async_trait]
impl<B> FromRequest<B> for TokenExtensions
where
    B: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        Ok(req
            .extensions()
            .get::<Self>()
            .expect("Failed to get jwt secret extension")
            .clone())
    }
}

async fn not_found(method: Method, uri: Uri, err: io::Error) -> String {
    let msg = format!("Method {method} for route {uri} caused error {err}");
    debug!("{msg}");
    msg
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

async fn geolocation_info(net: NetworkData) -> impl IntoResponse {
    debug!("Connection from {net:#?}");
    Json(net)
}
async fn user_agent_info(ua: UserAgentData) -> impl IntoResponse {
    debug!("User agent {ua:#?}");
    Json(ua)
}

use axum::extract::connect_info::Connected;
use hyper::server::conn::AddrStream;
use sqlx::types::ipnetwork::IpNetwork;

#[derive(Clone)]
pub struct ClientAddr(IpNetwork);

impl ClientAddr {
    pub fn network(&self) -> IpNetwork {
        self.0
    }
}

impl Connected<&AddrStream> for ClientAddr {
    fn connect_info(target: &AddrStream) -> Self {
        Self(IpNetwork::from(target.remote_addr().ip()))
    }
}

#[derive(Debug, Serialize)]
pub struct NetworkData {
    ip: IpAddr,
    geolocation_data: GeolocationData,
}

#[async_trait]
impl<B> FromRequest<B> for NetworkData
where
    B: Send + std::marker::Sync,
{
    type Rejection = hyper::StatusCode;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let ip = req
            .extract::<ConnectInfo<ClientAddr>>()
            .await
            .map_err(|e| {
                error!("Faield to get client ip");
                e.into_response().status()
            })?
            .0
            .network()
            .ip();

        if let Some(http_client) = req.extensions().get::<HttpClient>() {
            let geolocation_data = http_client.fetch_geolocation(ip).await.map_err(|e| {
                error!("Faield to fetch geolocation: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
            return Ok(Self {
                ip,
                geolocation_data,
            });
        } else {
            error!("Failed to get http client");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

#[async_trait]
impl<B> FromRequest<B> for UserAgentData
where
    B: Send + std::marker::Sync,
{
    type Rejection = hyper::StatusCode;

    async fn from_request(req: &mut RequestParts<B>) -> Result<Self, Self::Rejection> {
        let http_client = req
            .extensions()
            .get::<HttpClient>()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        let user_agent_header = req
            .headers()
            .get(http::header::USER_AGENT)
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        let user_agent_data = http_client
            .parse_user_agent(user_agent_header.to_str().unwrap())
            .await
            .unwrap();

        Ok(user_agent_data)
    }
}
