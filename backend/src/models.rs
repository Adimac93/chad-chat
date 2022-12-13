use crate::{utils::auth::errors::*, JwtSecret, RefreshJwtSecret};
use anyhow::Context;
use axum::{
    async_trait,
    extract::{self, FromRequest, RequestParts}, http::Extensions,
};
use axum_extra::extract::{CookieJar, cookie::{Cookie, SameSite}};
use dashmap::DashMap;
use jsonwebtoken::{decode, DecodingKey, Validation, encode, Header, EncodingKey};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use std::collections::{HashMap, HashSet};
use time::{OffsetDateTime, Duration};
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;
use validator::Validate;

#[async_trait]
pub trait AuthToken {
    async fn generate_cookie<'a>(token: String) -> Cookie<'a>;
    async fn generate_jwt(user_id: Uuid, login: &str, duration: Duration, key: &Secret<String>) -> Result<String, AuthError>;
    async fn get_jwt_key(ext: &Extensions) -> Secret<String>;
    async fn get_jwt_cookie(jar: CookieJar) -> Result<Cookie<'static>, AuthError>;
    async fn decode_jwt(token: &str, key: Secret<String>) -> Result<Self, AuthError> where Self: Sized;
    async fn check_if_in_blacklist(&self, pool: &PgPool) -> Result<bool, AuthError>;
    async fn add_token_to_blacklist(&self, pool: &PgPool) -> Result<(), AuthError>;
}

#[async_trait]
impl AuthToken for Claims {
    async fn get_jwt_key(ext: &Extensions) -> Secret<String> {
        let JwtSecret(jwt_key) = ext
            .get::<JwtSecret>()
            .expect("Failed to get jwt secret extension")
            .clone();

        jwt_key
    }

    async fn get_jwt_cookie(jar: CookieJar) -> Result<Cookie<'static>, AuthError> {
        jar.get("jwt").ok_or(AuthError::InvalidToken).cloned()
    }

    async fn decode_jwt(token: &str, key: Secret<String>) -> Result<Self, AuthError> {
        // decode token - validation setup
        let mut validation = Validation::default();
        validation.leeway = 5;

        // decode token - try to decode token with a provided jwt key
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(key.expose_secret().as_bytes()),
            &validation,
        ).map_err(|_e| AuthError::InvalidToken)?;

        Ok(data.claims)
    }

    async fn check_if_in_blacklist(&self, pool: &PgPool) -> Result<bool, AuthError> {
        // verify blacklist
        Ok(query!(
            r#"
                select * from jwt_blacklist
                where token_id = $1;
            "#,
            self.jti
        )
        .fetch_optional(pool)
        .await
        .context("Failed to verify token with the blacklist")?.is_some())
    }

    async fn generate_cookie<'a>(token: String) -> Cookie<'a> {
        Cookie::build(String::from("jwt"), token)
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .path("/")
            .finish()
    }

    async fn generate_jwt(
        user_id: Uuid,
        login: &str,
        duration: Duration,
        key: &Secret<String>
    ) -> Result<String, AuthError> {
        let claims = Claims::new(user_id, login, duration);
    
        Ok(encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(key.expose_secret().as_bytes()),
        )
        .context("Failed to encrypt token")?)
    }

    async fn add_token_to_blacklist(&self, pool: &PgPool) -> Result<(), AuthError> {
        let exp = OffsetDateTime::from_unix_timestamp(self.exp as i64)
            .context("Failed to convert timestamp to date and time with the timezone")?;
    
        let _res = query!(
            r#"
                insert into jwt_blacklist (token_id, expiry)
                values ($1, $2)
            "#,
            self.jti,
            exp,
        )
        .execute(pool)
        .await
        .context("Failed to add token to the blacklist")?;
        
        Ok(())
    }
}

#[async_trait]
impl AuthToken for RefreshClaims {
    async fn get_jwt_key(ext: &Extensions) -> Secret<String> {
        let RefreshJwtSecret(jwt_key) = ext
            .get::<RefreshJwtSecret>()
            .expect("Failed to get jwt secret extension")
            .clone();

        jwt_key
    }

    async fn get_jwt_cookie(jar: CookieJar) -> Result<Cookie<'static>, AuthError> {
        jar.get("refresh-jwt").ok_or(AuthError::InvalidToken).cloned()
    }

    async fn decode_jwt(token: &str, key: Secret<String>) -> Result<Self, AuthError> {
        // decode token - validation setup
        let mut validation = Validation::default();
        validation.leeway = 5;

        // decode token - try to decode token with a provided jwt key
        let data = decode::<RefreshClaims>(
            token,
            &DecodingKey::from_secret(key.expose_secret().as_bytes()),
            &validation,
        ).map_err(|_e| AuthError::InvalidToken)?;

        Ok(data.claims)
    }

    async fn check_if_in_blacklist(&self, pool: &PgPool) -> Result<bool, AuthError> {
        // verify blacklist
        Ok(query!(
            r#"
                select * from jwt_blacklist
                where token_id = $1;
            "#,
            self.jti
        )
        .fetch_optional(pool)
        .await
        .context("Failed to verify token with the blacklist")?.is_some())
    }

    async fn generate_cookie<'a>(token: String) -> Cookie<'a> {
        Cookie::build(String::from("refresh-jwt"), token)
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .path("/")
            .finish()
    }

    async fn generate_jwt(
        user_id: Uuid,
        login: &str,
        duration: Duration,
        key: &Secret<String>
    ) -> Result<String, AuthError> {
        let refresh_claims = RefreshClaims::new(user_id, login, duration);
    
        Ok(encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(key.expose_secret().as_bytes()),
        )
        .context("Failed to encrypt token")?)
    }

    async fn add_token_to_blacklist(&self, pool: &PgPool) -> Result<(), AuthError> {
        let exp = OffsetDateTime::from_unix_timestamp(self.exp as i64)
            .context("Failed to convert timestamp to date and time with the timezone")?;
    
        let _res = query!(
            r#"
                insert into jwt_blacklist (token_id, expiry)
                values ($1, $2)
            "#,
            self.jti,
            exp,
        )
        .execute(pool)
        .await
        .context("Failed to add token to the blacklist")?;
        
        Ok(())
    }
}


#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub jti: Uuid,
    pub user_id: Uuid,
    pub login: String,
    pub exp: u64,
}

impl Claims {
    pub fn new(user_id: Uuid, login: &str, duration: Duration) -> Self {
        Self {
            jti: Uuid::new_v4(),
            user_id,
            login: login.to_string(),
            exp: jsonwebtoken::get_current_timestamp() + duration.whole_seconds().abs() as u64,
        }   
    }
}

#[async_trait]
impl<B> FromRequest<B> for Claims
where
    B: Send,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        verify_token::<Self, B>(req).await
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshClaims {
    pub jti: Uuid,
    pub user_id: Uuid,
    pub login: String,
    pub exp: u64,
}

impl RefreshClaims {
    pub fn new(user_id: Uuid, login: &str, duration: Duration) -> Self {
        Self {
            jti: Uuid::new_v4(),
            user_id,
            login: login.to_string(),
            exp: jsonwebtoken::get_current_timestamp() + duration.whole_seconds().abs() as u64,
        }
    }
}

#[async_trait]
impl<B> FromRequest<B> for RefreshClaims
where
    B: Send,
{
    type Rejection = AuthError;

    async fn from_request(req: &mut extract::RequestParts<B>) -> Result<Self, Self::Rejection> {
        verify_token::<Self, B>(req).await
    }
}

async fn verify_token<T, B>(req: &mut RequestParts<B>) -> Result<T, AuthError>
where 
T: AuthToken,
B: Send {
    // get extensions
    let ext = req.extensions();

    let jwt_key = T::get_jwt_key(ext).await;

    // get extensions - PgPool
    let pool = ext
        .get::<PgPool>()
        .expect("Failed to get PgPool to check jwt claims")
        .clone();

    // get extensions - CookieJar
    let jar = CookieJar::from_request(req)
        .await
        .context("Failed to fetch cookie jar")?;

    let cookie = T::get_jwt_cookie(jar).await?;

    let claims = T::decode_jwt(cookie.value(), jwt_key).await?;

    let res = claims.check_if_in_blacklist(&pool).await?;

    match res {
        true => Err(AuthError::InvalidToken),
        false => Ok(claims),
    }
}

#[derive(Serialize, Deserialize, Validate)]
pub struct LoginCredentials {
    pub login: String,
    pub password: String,
}

impl LoginCredentials {
    pub fn new(login: &str, password: &str) -> Self {
        Self {
            login: login.into(),
            password: password.into(),
        }
    }
}


#[derive(Serialize, Deserialize, Validate)]
pub struct RegisterCredentials {
    #[validate(length(min = 4, max = 20, message = "Invalid username length"), does_not_contain(pattern = " ", message = "Username contains spaces"))]
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
