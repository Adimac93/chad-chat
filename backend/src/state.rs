use crate::{
    configuration::Settings,
    modules::{
        database::get_postgres_pool,
        external_api::HttpClient,
        extractors::jwt::{JwtAccessSecret, JwtRefreshSecret, TokenExtractors},
        smtp::Mailer,
    },
    utils::{
        chat::socket::ChatState,
        roles::models::{Gate, Role},
    },
};
use axum::extract::FromRef;
use sqlx::PgPool;
use uuid::Uuid;

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

pub fn is_id_the_same(val: (Uuid, Uuid)) -> bool {
    val.0 == val.1
}
