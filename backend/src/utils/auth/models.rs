use std::collections::HashMap;

use crate::{
    errors::AppError,
    modules::extractors::jwt::{JwtAccessSecret, JwtRefreshSecret},
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
use redis::{Cmd, Value, ConnectionLike, Pipeline};
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

    let token_in_blacklist = get_access_token_from_blacklist(&mut state.redis.clone(), claims.user_id, claims.jti).await?;

    if token_in_blacklist == Value::Nil {
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

pub async fn add_access_token_to_blacklist<'c>(pipe: &mut Pipeline, claims: Claims) -> Result<(), AppError> {
    // this is converted into the transaction
    // performs an insert to add an access token to the blacklist

    pipe.hset(&format!("tokens:{}:blacklist", claims.user_id), claims.jti.as_bytes(), claims.exp);
    
    Ok(())
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

pub async fn get_access_token_from_blacklist(rd: &mut RdPool, user_id: Uuid, jti: Uuid) -> Result<Value, AppError> {
    let val = Cmd::hget(&format!("tokens:{}:blacklist", user_id), jti.as_bytes()).query_async(rd).await.context("Failed to get the token from the blacklist")?;
    Ok(val)
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

    let mut validation = Validation::default();
    validation.leeway = 5;

    let cookie = jar.get("refresh-jwt").ok_or(AppError::exp(StatusCode::UNAUTHORIZED, "No refresh token found"))?;
    let decoding_key = DecodingKey::from_secret(jwt_key.expose_secret().as_bytes());

    let claims: RefreshClaims = decode(
        cookie.value(),
        &decoding_key,
        &validation,
    ).context("Invalid or expired token")?.claims;

    let mut rd = state.redis.clone();
    let token_in_blacklist = get_refresh_token_from_blacklist(&mut rd, claims).await?;

    if token_in_blacklist == Value::Nil {
        Ok(claims)
    } else {
        invalidate_all_refresh_tokens(&mut rd, claims.user_id).await?;
        Err(AppError::exp(StatusCode::UNAUTHORIZED, "Invalid token"))
    }
}

pub fn validate_refresh_token<'a>(cookie: &Cookie<'a>, secret: &Secret<String>) -> Result<RefreshClaims, AppError> {
    let mut validation = Validation::default();
    validation.leeway = 5;

    let decoding_key = DecodingKey::from_secret(secret.expose_secret().as_bytes());

    let claims: RefreshClaims = decode(
        cookie.value(),
        &decoding_key,
        &validation,
    ).context("Invalid or expired token")?.claims;

    Ok(claims)
}

pub async fn add_refresh_token_to_blacklist<'c>(pipe: &mut Pipeline, claims: RefreshClaims) -> Result<(), AppError> {
    pipe.hdel(&format!("tokens:{}:whitelist", claims.user_id), claims.jti.as_bytes());
    pipe.hset(&format!("tokens:{}:blacklist", claims.user_id), claims.jti.as_bytes(), claims.exp);

    Ok(())
}

pub async fn setup_refresh_token<'a>(rd: &mut RdPool, user_id: Uuid, ext: &JwtRefreshSecret) -> Result<Cookie<'a>, AppError> {
    let claims = RefreshClaims::new(user_id, Duration::days(7));

    add_refresh_token_to_whitelist(rd, claims).await?;
    let cookie = create_refresh_token(claims, ext)?;

    Ok(cookie)
}

pub async fn add_refresh_token_to_whitelist(rd: &mut RdPool, claims: RefreshClaims) -> Result<(), AppError> {
    Cmd::hset(&format!("tokens:{}:whitelist", claims.user_id), claims.jti.as_bytes(), claims.exp).query_async(rd).await.context("Failed to add refresh token to whitelist")?;
    Ok(())
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

pub async fn get_refresh_token_from_blacklist(rd: &mut RdPool, claims: RefreshClaims) -> Result<Value, AppError> {
    let val = Cmd::hget(&format!("tokens:{}:blacklist", claims.user_id), claims.jti.as_bytes()).query_async(rd).await.context("Failed to get the token from the blacklist")?;
    Ok(val)
}

pub async fn invalidate_all_refresh_tokens(rd: &mut RdPool, user_id: Uuid) -> Result<(), AppError> {
    let val: HashMap<String, i64> = Cmd::hgetall(&format!("tokens:{}:whitelist", user_id)).query_async(rd).await.context("Failed to read tokens from the whitelist")?;
    
    let mut pipe = Pipeline::new();
    let atomic_pipe = pipe.atomic();
    atomic_pipe.hset_multiple(&format!("tokens:{}:blacklist", user_id), &val.into_iter().collect::<Vec<(String, i64)>>());
    atomic_pipe.del(&format!("tokens:{}:whitelist", user_id));
    atomic_pipe.query_async(rd).await.context("Failed to transfer refresh tokens to the blacklist")?;
    
    Ok(())
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
