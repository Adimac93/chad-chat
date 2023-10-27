pub mod additions;
pub mod models;
pub mod tokens;
use crate::errors::AppError;
use crate::modules::{extractors::jwt::TokenExtractors, smtp::Mailer};
use anyhow::Context;
use argon2::verify_encoded;
use axum_extra::extract::{cookie::Cookie, CookieJar};
use hyper::StatusCode;
use models::*;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::{query, types::ipnetwork::IpNetwork, PgPool};
use time::OffsetDateTime;
use tracing::{debug, trace};
use uuid::Uuid;
use validator::Validate;

use self::additions::random_username_tag;

#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
#[sqlx(type_name = "status", rename_all = "snake_case")]
pub enum ActivityStatus {
    Online,
    Offline,
    Idle,
}

// todo: make as transaction with Acquire
pub async fn try_register_user<'c>(
    pool: &PgPool,
    ip: IpNetwork,
    mailer: Option<Mailer>,
    email: &str,
    password: SecretString,
    username: &str,
) -> Result<Uuid, AppError> {
    let mut transaction = pool.begin().await?;

    let user = query!(
        r#"
            SELECT id FROM credentials WHERE email = $1
        "#,
        email
    )
    .fetch_optional(&mut *transaction)
    .await?;

    if user.is_some() {
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "User already exists",
        ));
    }

    if email.trim().is_empty() || password.expose_secret().trim().is_empty() {
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Missing email or password",
        ));
    }

    let _ = RegisterCredentials::new(email, password.expose_secret(), &username)
        .validate()
        .map_err(|e| AppError::exp(StatusCode::BAD_REQUEST, &format!("Invalid email: {e}")))?;

    if !additions::pass_is_strong(password.expose_secret(), &[&email]) {
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Password is too weak",
        ));
    }

    let hashed_pass = additions::hash_pass(password)
        .context("Failed to hash password with argon2")
        .map_err(AppError::Unexpected)?;

    let mut username = username.trim();
    if username.is_empty() {
        // TODO: Generate random username
        username = "I am definitely not a chad"
    }

    let used_tags = query!(
        r#"
            SELECT tag FROM users
            WHERE username = $1
        "#,
        username
    )
    .fetch_all(&mut *transaction)
    .await?;

    let tag = random_username_tag(used_tags.into_iter().map(|record| record.tag).collect()).ok_or(
        AppError::exp(
            StatusCode::BAD_REQUEST,
            "Maximum number of tags for this username",
        ),
    )?;

    let user_id = query!(
        r#"
            INSERT INTO users (username, tag, activity_status)
            VALUES ($1, $2, $3)
            RETURNING (id)
        "#,
        username,
        tag,
        ActivityStatus::Online as ActivityStatus,
    )
    .fetch_one(&mut *transaction)
    .await?
    .id;

    query!(
        r#"
            INSERT INTO credentials (id, email, password)
            VALUES ($1, $2, $3)
        "#,
        user_id,
        email,
        hashed_pass
    )
    .execute(&mut *transaction)
    .await?;

    transaction.commit().await?;

    Ok(user_id)
}

pub async fn verify_user_credentials(
    pool: &PgPool,
    email: &str,
    password: SecretString,
) -> Result<Uuid, AppError> {
    debug!("Verifying credentials");
    if email.trim().is_empty() || password.expose_secret().trim().is_empty() {
        return Err(AppError::exp(
            StatusCode::BAD_REQUEST,
            "Missing email or password",
        ))?;
    }

    let res = query!(
        r#"
            SELECT id, password FROM credentials
            WHERE email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::exp(
        StatusCode::UNAUTHORIZED,
        "Incorrect email or password",
    ))?;

    match verify_encoded(&res.password, password.expose_secret().as_bytes())
        .context("Failed to verify credentials")
        .map_err(AppError::Unexpected)?
    {
        true => Ok(res.id),
        false => Err(AppError::exp(
            StatusCode::UNAUTHORIZED,
            "Incorrect email or password",
        )),
    }
}

pub async fn generate_token_cookies(
    user_id: Uuid,
    login: &str,
    ext: &TokenExtractors,
    jar: CookieJar,
) -> Result<CookieJar, AppError> {
    let access_cookie = generate_jwt_in_cookie::<Claims>(user_id, login, ext).await?;

    trace!("Access JWT: {access_cookie:#?}");

    let refresh_cookie = generate_jwt_in_cookie::<RefreshClaims>(user_id, login, ext).await?;

    trace!("Refresh JWT: {refresh_cookie:#?}");

    Ok(jar.add(access_cookie).add(refresh_cookie))
}

async fn generate_jwt_in_cookie<'a, T>(
    user_id: Uuid,
    login: &str,
    ext: &TokenExtractors,
) -> Result<Cookie<'a>, AppError>
where
    T: AuthToken,
{
    let access_token = T::generate_jwt(
        user_id,
        login,
        T::JWT_EXPIRATION,
        &T::get_jwt_key(ext).await,
    )
    .await?;

    let access_cookie = T::generate_cookie(access_token).await;
    trace!("Access JWT: {access_cookie:#?}");

    Ok(access_cookie)
}

pub async fn add_token_to_blacklist(pool: &PgPool, claims: &Claims) -> Result<(), AppError> {
    let exp = OffsetDateTime::from_unix_timestamp(claims.exp as i64)
        .context("Failed to convert timestamp to date and time with the timezone")
        .map_err(AppError::Unexpected)?;

    let _res = query!(
        r#"
            INSERT INTO jwt_blacklist (token_id, expiry)
            VALUES ($1, $2)
        "#,
        claims.jti,
        exp,
    )
    .execute(pool)
    .await?;

    trace!("Adding token to blacklist");
    Ok(())
}
