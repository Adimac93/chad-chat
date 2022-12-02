﻿use crate::{utils::auth::errors::*, JwtSecret};
use anyhow::Context;
use axum::{
    async_trait,
    extract::{self, FromRequest}, Extension,
};
use axum_extra::extract::CookieJar;
use dashmap::DashMap;
use jsonwebtoken::{decode, DecodingKey, Validation};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use std::collections::{HashMap, HashSet};
use time::OffsetDateTime;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub jti: Uuid,
    pub user_id: Uuid,
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

        let pool = ext
            .get::<PgPool>()
            .expect("Failed to get PgPool to check jwt claims")
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

        let data = data.map_err(|_| AuthError::InvalidToken)?;

        let res = query!(
            r#"
                select * from jwt_blacklist
                where token_id = $1;
            "#,
            data.claims.jti
        )
        .fetch_optional(&pool)
        .await
        .context("Failed to verify token with the blacklist")?;

        match res {
            Some(_) => Err(AuthError::InvalidToken),
            None => Ok(data.claims),
        }
    }
}

#[derive(Serialize, Deserialize, Validate)]
pub struct LoginCredentials {
    pub login: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Validate)]
pub struct RegisterCredentials {
    #[validate(length(min = 4, max = 20), does_not_contain = " ")]
    pub login: String,
    pub password: String,
    pub nickname: String,
}

impl RegisterCredentials {
    pub fn new(login: &str, password: &str, nickname: &str) -> Self {
        Self {
            login: login.into(),
            password: password.into(),
            nickname: nickname.into(),
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
