pub mod additions;
pub mod errors;
use crate::{
    models::{Claims, LoginCredentials, RegisterCredentials, RefreshClaims},
    routes::auth::{JWT_ACCESS_TOKEN_EXPIRATION, JWT_REFRESH_TOKEN_EXPIRATION},
};
use anyhow::{Context, Error};
use argon2::verify_encoded;
use axum_extra::extract::{cookie::{Cookie, SameSite}, CookieJar};
use errors::*;
use jsonwebtoken::{encode, EncodingKey, Header};
use secrecy::{ExposeSecret, Secret, SecretString};
use serde::{Serialize, Deserialize};
use sqlx::{query, PgPool};
use time::Duration;
use tracing::info;
use tracing::debug;
use uuid::Uuid;
use validator::Validate;
use time::OffsetDateTime;

pub async fn try_register_user(
    pool: &PgPool,
    login: &str,
    password: SecretString,
    nickname: &str,
) -> Result<Uuid, AuthError> {
    let user = query!(
        r#"
            select * from users where login = $1
        "#,
        login
    )
    .fetch_optional(pool)
    .await
    .context("Failed to query user by login")?;

    if user.is_some() {
        return Err(AuthError::UserAlreadyExists);
    }

    if login.trim().is_empty() || password.expose_secret().trim().is_empty() {
        return Err(AuthError::MissingCredential);
    }

    let _ = RegisterCredentials::new(login, password.expose_secret(), &nickname).validate()?;

    if !additions::pass_is_strong(password.expose_secret(), &[&login]) {
        return Err(AuthError::WeakPassword);
    }

    let hashed_pass = additions::hash_pass(password).context("Failed to hash pass")?;

    let mut nickname = nickname.trim();
    if nickname.is_empty() {
        // TODO: Generate random nickname
        nickname = "I am definitely not a chad"
    }

    let user_id = query!(
        r#"
            insert into users (login, password, nickname)
            values ($1, $2, $3)
            returning (id)
        "#,
        login,
        hashed_pass,
        nickname
    )
    .fetch_one(pool)
    .await
    .context("Failed to create a new user")?
    .id;

    info!("{user_id:?}");

    Ok(user_id)
}

pub async fn verify_user_credentials (
    pool: &PgPool,
    login: &str,
    password: SecretString,
) -> Result<Uuid, AuthError> {
    debug!("Verifying credentials");
    if login.trim().is_empty() || password.expose_secret().trim().is_empty() {
        return Err(AuthError::MissingCredential);
    }

    let res = query!(
        r#"
            select * from users where login = $1
        "#,
        login
    )
    .fetch_optional(pool)
    .await
    .context("Failed to select user by login")?
    .ok_or(AuthError::WrongUserOrPassword)?;

    match verify_encoded(&res.password, password.expose_secret().as_bytes())
        .context("Failed to verify password")?
    {
        true => Ok(res.id),
        false => Err(AuthError::WrongUserOrPassword),
    }
}

pub async fn login_user (
    user_id: Uuid,
    user: &LoginCredentials,
    jwt_key: Secret<String>,
    refresh_jwt_key: Secret<String>,
    jar: CookieJar
) -> Result<CookieJar, AuthError> {
    let access_token =
        generate_jwt_token(user_id, &user.login, JWT_ACCESS_TOKEN_EXPIRATION, &jwt_key).await?;
    let access_cookie = generate_cookie(access_token, JwtTokenType::Access).await;

    let refresh_token =
        generate_refresh_jwt_token(user_id, &user.login, JWT_REFRESH_TOKEN_EXPIRATION, &refresh_jwt_key)
            .await?;
    let refresh_cookie = generate_cookie(refresh_token, JwtTokenType::Refresh).await;

    let jar = jar.add(access_cookie);
    Ok(jar.add(refresh_cookie))
}

pub async fn generate_jwt_token(
    user_id: Uuid,
    login: &str,
    duration: Duration,
    key: &Secret<String>
) -> Result<String, Error> {
    debug!("Trying to generate jwt token");
    let claims = Claims::new(user_id, login, duration);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(key.expose_secret().as_bytes()),
    )
    .context("Failed to encrypt token")
}

pub async fn generate_refresh_jwt_token(
    user_id: Uuid,
    login: &str,
    duration: Duration,
    key: &Secret<String>
) -> Result<String, Error> {
    debug!("Trying to generate refresh jwt token");
    let claims = RefreshClaims::new(user_id, login, duration);

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(key.expose_secret().as_bytes()),
    )
    .context("Failed to encrypt token")
}

pub async fn add_token_to_blacklist(pool: &PgPool, claims: &Claims) -> Result<(), AuthError> {
    let exp = OffsetDateTime::from_unix_timestamp(claims.exp as i64)
        .context("Failed to convert timestamp to date and time with the timezone")?;

    let _res = query!(
        r#"
            insert into jwt_blacklist (token_id, expiry)
            values ($1, $2)
        "#,
        claims.jti,
        exp,
    )
    .execute(pool)
    .await
    .context("Failed to add token to the blacklist")?;
    
    Ok(())
}

pub async fn generate_cookie<'a>(token: String, token_type: JwtTokenType) -> Cookie<'a> {
    debug!("Generating a cookie");
    Cookie::build(String::from(token_type), token)
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .finish()
}

#[derive(Serialize, Deserialize, Debug)]
pub enum JwtTokenType {
    Access,
    Refresh,
}

impl From<JwtTokenType> for String {
    fn from(token_type: JwtTokenType) -> Self {
        match token_type {
            JwtTokenType::Access => "jwt".into(),
            JwtTokenType::Refresh => "refresh-jwt".into(),
        }
    }
}
