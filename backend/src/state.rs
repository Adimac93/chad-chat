use crate::{
    configuration::Settings,
    modules::{
        database::{get_postgres_pool, get_redis_pool},
        external_api::HttpClient,
        extractors::jwt::{JwtAccessSecret, JwtRefreshSecret, TokenExtractors},
        smtp::Mailer,
    },
    utils::chat::socket::ChatState,
};
use axum::extract::FromRef;
use redis::aio::ConnectionManager;
use sqlx::PgPool;
use uuid::Uuid;

pub type RdPool = ConnectionManager;

#[derive(FromRef, Clone)]
pub struct AppState {
    pub postgres: PgPool,
    pub redis: RdPool,
    pub client: HttpClient,
    pub smtp: Mailer,
    pub token_ext: TokenExtractors,
    pub chat_state: ChatState,
}

impl AppState {
    pub async fn new(config: Settings, test_pool: Option<PgPool>) -> Self {
        let token_ext = TokenExtractors {
            access: JwtAccessSecret(config.app.access_jwt_secret),
            refresh: JwtRefreshSecret(config.app.refresh_jwt_secret),
        };

        let chat_state = ChatState::new();

        AppState {
            postgres: test_pool.unwrap_or(get_postgres_pool(config.postgres).await),
            redis: get_redis_pool(config.redis).await,
            client: HttpClient::new(),
            smtp: Mailer::new(config.smtp, config.app.origin),
            token_ext,
            chat_state,
        }
    }
}

pub fn is_id_the_same(val: (Uuid, Uuid)) -> bool {
    val.0 == val.1
}
