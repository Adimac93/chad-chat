use std::collections::HashMap;

use crate::{
    errors::AppError,
    modules::{extractors::jwt::{JwtAccessSecret, JwtRefreshSecret}, redis_tools::{CacheWrite, redis_path::RedisRoot, CacheRead, CacheInvalidate, execute_commands}},
    state::{AppState, RdPool},
};
use anyhow::Context;
use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use hyper::StatusCode;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use redis::{Cmd, Value, Pipeline, RedisError};
use redis::aio::ConnectionLike;
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::{query, Acquire, Postgres};
use time::{Duration, OffsetDateTime};
use typeshare::typeshare;
use uuid::Uuid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Claims {
    pub jti: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub exp: u64,
}

impl Claims {
    pub fn new(user_id: Uuid, email: String, duration: Duration) -> Self {
        Self {
            jti: Uuid::new_v4(),
            user_id,
            email,
            exp: jsonwebtoken::get_current_timestamp() + duration.whole_seconds().unsigned_abs(),
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for Claims {
    type Rejection = AppError;

    async fn from_request_parts(
        req: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        verify_access_token(req, state).await
    }
}

pub async fn verify_access_token(req: &mut Parts, state: &AppState) -> Result<Claims, AppError> {
    // JWT tokens can be of one of four states: valid, fake, invalidated (blacklisted), expired.
    // first, the token is passed through the verification function provided by the crate to weed out fake and expired tokens from the rest
    // second, the token is compared with the blacklist to ensure that the token is not invalidated

    let JwtAccessSecret(jwt_key) = &state.token_ext.access;
    let jar = CookieJar::from_request_parts(req, state)
        .await
        .context("Failed to fetch cookie jar")?;

    let cookie = jar.get("jwt").ok_or(AppError::exp(StatusCode::UNAUTHORIZED, "No access token found"))?;
    let claims = validate_access_token(cookie, jwt_key)?;

    let token_in_blacklist = TokenBlacklist::new(claims.user_id, claims.jti).read(&mut state.redis.clone()).await?;

    if token_in_blacklist.is_none() {
        Ok(claims)
    } else {
        Err(AppError::exp(StatusCode::UNAUTHORIZED, "Invalid token"))
    }
}

pub fn validate_access_token<'a>(cookie: &Cookie<'a>, secret: &Secret<String>) -> Result<Claims, AppError> {
    let mut validation = Validation::default();
    validation.leeway = 5;

    let decoding_key = DecodingKey::from_secret(secret.expose_secret().as_bytes());

    let claims: Claims = decode(
        cookie.value(),
        &decoding_key,
        &validation,
    ).context("Invalid or expired token")?.claims;

    Ok(claims)
}

pub async fn create_access_token<'a>(user_id: Uuid, email: String, ext: &JwtAccessSecret) -> Result<Cookie<'a>, AppError> {
    // use credentials to create a new fresh access JWT
    // generate a cookie with key `jwt`
    // set the value to that encoded access JWT

    let claims = Claims::new(user_id, email, Duration::seconds(15));

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(ext.0.expose_secret().as_bytes()),
    ).context("Failed to encode the access JWT")?;

    let cookie = Cookie::build(String::from("jwt"), token)
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .finish();
    
    Ok(cookie)
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct RefreshClaims {
    pub jti: Uuid,
    pub user_id: Uuid,
    pub exp: u64,
}

impl RefreshClaims {
    pub fn new(user_id: Uuid, duration: Duration) -> Self {
        Self {
            jti: Uuid::new_v4(),
            user_id,
            exp: jsonwebtoken::get_current_timestamp() + duration.whole_seconds().unsigned_abs(),
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for RefreshClaims {
    type Rejection = AppError;

    async fn from_request_parts(
        req: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        verify_refresh_token(req, state).await
    }
}

pub async fn verify_refresh_token(req: &mut Parts, state: &AppState) -> Result<RefreshClaims, AppError> {
    let JwtRefreshSecret(jwt_key) = &state.token_ext.refresh;
    let jar = CookieJar::from_request_parts(req, state)
        .await
        .context("Failed to fetch cookie jar")?;

    let cookie = jar.get("refresh-jwt").ok_or(AppError::exp(StatusCode::UNAUTHORIZED, "No refresh token found"))?;

    let claims = validate_refresh_token(cookie.value(), jwt_key)?;

    let mut rd = state.redis.clone();
    let token_in_blacklist = TokenBlacklist::new(claims.user_id, claims.jti).read(&mut rd).await?;

    if token_in_blacklist.is_none() {
        Ok(claims)
    } else {
        TokenWhitelist::new(claims.user_id, claims.jti).invalidate_all_tokens(&mut rd).await?;
        Err(AppError::exp(StatusCode::UNAUTHORIZED, "Invalid token"))
    }
}

pub fn validate_refresh_token<'a>(token: &str, secret: &Secret<String>) -> Result<RefreshClaims, AppError> {
    let mut validation = Validation::default();
    validation.leeway = 5;

    let decoding_key = DecodingKey::from_secret(secret.expose_secret().as_bytes());

    let claims: RefreshClaims = decode(
        token,
        &decoding_key,
        &validation,
    ).context("Invalid or expired token")?.claims;

    Ok(claims)
}

pub async fn setup_refresh_token<'a>(rd: &mut RdPool, user_id: Uuid, ext: &JwtRefreshSecret) -> Result<Cookie<'a>, AppError> {
    let claims = RefreshClaims::new(user_id, Duration::days(7));

    TokenWhitelist::new(user_id, claims.jti).write(rd, claims.exp).await?;
    let cookie = create_refresh_token(claims, ext)?;

    Ok(cookie)
}

pub fn create_refresh_token<'a>(claims: RefreshClaims, ext: &JwtRefreshSecret) -> Result<Cookie<'a>, AppError> {
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(ext.0.expose_secret().as_bytes()),
    ).context("Failed to encode the refresh JWT")?;

    let cookie = Cookie::build(String::from("refresh-jwt"), token)
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .finish();
    
    Ok(cookie)
}

pub struct TokenBlacklist {
    user_id: Uuid,
    jti: Uuid,
}

impl TokenBlacklist {
    pub fn new(user_id: Uuid, jti: Uuid) -> Self {
        Self { user_id, jti, }
    }
}

impl CacheWrite for TokenBlacklist {
    type Stored = u64;

    fn write_cmd(&self, exp: Self::Stored) -> Vec<Cmd> {
        vec![Cmd::hset(RedisRoot.tokens(self.user_id).blacklist().to_string(), self.jti.as_bytes(), exp)]
    }
}

impl CacheRead for TokenBlacklist {
    type Stored = u64;

    fn read_cmd(&self) -> Vec<Cmd> {
        vec![Cmd::hget(RedisRoot.tokens(self.user_id).blacklist().to_string(), self.jti.as_bytes())]
    }
}

pub struct TokenWhitelist {
    user_id: Uuid,
    jti: Uuid,
}

impl TokenWhitelist {
    pub fn new(user_id: Uuid, jti: Uuid) -> Self {
        Self { user_id, jti, }
    }

    pub async fn read_all_tokens(&self, rd: &mut impl ConnectionLike) -> Result<HashMap<u128, u64>, RedisError> {
        let res: HashMap<u128, u64> = Cmd::hgetall(RedisRoot.tokens(self.user_id).to_string()).query_async(rd).await?;
        Ok(res)
    }

    pub async fn invalidate_all_tokens(&self, rd: &mut impl ConnectionLike) -> Result<(), RedisError> {
        let tokens = self.read_all_tokens(rd).await?;
        
        let mut cmds: Vec<Cmd> = tokens.into_iter().fold(vec![], |mut cmds, (jti, exp)| {
            cmds.extend(TokenBlacklist::new(self.user_id, Uuid::from_u128(jti)).write_cmd(exp));
            cmds
        });
        cmds.push(Cmd::del(RedisRoot.tokens(self.user_id).whitelist().to_string()));
        
        let _ = execute_commands(rd, cmds).await?;
        Ok(())
    }

    pub async fn move_token_to_blacklist(&self, rd: &mut impl ConnectionLike, exp: u64) -> Result<(), AppError> {
        let mut cmds = self.invalidate_cmd();
        cmds.extend(TokenBlacklist::new(self.user_id, self.jti).write_cmd(exp));
        let _ = execute_commands(rd, cmds).await?;
        Ok(())
    }
}

impl CacheWrite for TokenWhitelist {
    type Stored = u64;

    fn write_cmd(&self, exp: Self::Stored) -> Vec<Cmd> {
        vec![Cmd::hset(RedisRoot.tokens(self.user_id).blacklist().to_string(), self.jti.as_bytes(), exp)]
    }
}

impl CacheRead for TokenWhitelist {
    type Stored = i64;

    fn read_cmd(&self) -> Vec<Cmd> {
        vec![Cmd::hget(RedisRoot.tokens(self.user_id).blacklist().to_string(), self.jti.as_bytes())]
    }
}

impl CacheInvalidate for TokenWhitelist {
    fn invalidate_cmd(&self) -> Vec<Cmd> {
        vec![Cmd::hdel(RedisRoot.tokens(self.user_id).blacklist().to_string(), self.jti.as_bytes())]
    }
}

#[typeshare]
#[derive(Serialize, Deserialize, Validate)]
pub struct LoginCredentials {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

impl LoginCredentials {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.into(),
            password: password.into(),
        }
    }
}

#[typeshare]
#[derive(Serialize, Deserialize, Validate)]
pub struct RegisterCredentials {
    #[validate(email)]
    pub email: String,
    pub password: String,
    pub username: String,
}

impl RegisterCredentials {
    pub fn new(email: &str, password: &str, username: &str) -> Self {
        Self {
            email: email.into(),
            password: password.into(),
            username: username.into(),
        }
    }
}
