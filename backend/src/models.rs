use crate::{utils::auth::errors::*, JwtSecret};
use anyhow::Context;
use axum::{
    async_trait,
    extract::{self, FromRequest},
};
use axum_extra::extract::CookieJar;
use dashmap::DashMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub id: Uuid,
    pub login: String,
    pub exp: u64,
}

#[async_trait]
impl<B> FromRequest<B> for Claims
where
    B: Send,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        let ext = req.extensions();
        let JwtSecret(jwt_key) = ext
            .get::<JwtSecret>()
            .expect("Failed to get jwt secret extension")
            .clone();

        let jar = CookieJar::from_request(req)
            .await
            .context("Failed to fetch cookie jar")?;
        let cookie = jar.get("jwt").ok_or(AuthError::InvalidToken)?;
        let mut validation = Validation::default();
        validation.leeway = 5;

        let data = decode::<Claims>(
            cookie.value(),
            &DecodingKey::from_secret(jwt_key.expose_secret().as_bytes()),
            &validation,
        );
        let new_data = data.map_err(|_| AuthError::InvalidToken)?;
        Ok(new_data.claims)
    }
}

const MIN_USERNAME_LENGTH: u8 = 4;
const MAX_USERNAME_LENGTH: u8 = 20;

#[derive(Serialize, Deserialize, Validate)]
pub struct LoginCredentials {
    #[validate(length(min = "MIN_USERNAME_LENGTH", max = "MAX_USERNAME_LENGTH"), does_not_contain = " ")]
    pub login: String,
    pub password: String,
}

impl LoginCredentials {
    pub fn new(login: &str, password: &str) -> Self {
        Self {
            login: login.to_string(),
            password: password.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct NewGroup {
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Group {
    pub id: Uuid,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
pub struct GroupUser {
    pub user_id: Uuid,
    pub group_id: Uuid,
}

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub content: String,
    pub user_id: Uuid,
    pub group_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MessageModel {
    pub id: i32,
    pub content: String,
    pub user_id: Uuid,
    pub group_id: Uuid,
    pub sent_at: OffsetDateTime,
}

pub struct InvitationState {
    pub code: RwLock<HashMap<Uuid, Uuid>>,
    // invitation : group
}

impl InvitationState {
    pub fn new() -> Self {
        InvitationState {
            code: RwLock::new(HashMap::new()),
        }
    }
}

#[derive(Deserialize)]
pub struct NewGroupInvitation {
    pub group_id: Uuid,
}

pub struct ChatState {
    pub groups: DashMap<Uuid, GroupTransmitter>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            groups: DashMap::new(),
        }
    }
}

pub struct GroupTransmitter {
    pub tx: broadcast::Sender<String>,
    pub users: HashSet<Uuid>,
}

impl GroupTransmitter {
    pub fn new() -> Self {
        let (tx, _rx) = broadcast::channel(100);
        Self {
            tx,
            users: HashSet::new(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub struct GroupInfo {
    pub name: String,
    pub members: i64,
}
