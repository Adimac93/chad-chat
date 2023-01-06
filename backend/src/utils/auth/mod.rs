pub mod additions;
pub mod errors;
pub mod models;

use crate::{app_errors::AppError, TokenExtensions};
use anyhow::Context;
use argon2::verify_encoded;
use axum_extra::extract::{cookie::Cookie, CookieJar};
use errors::*;
use models::*;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, Acquire, PgPool, Postgres};
use time::OffsetDateTime;
use tracing::{debug, trace};
use uuid::Uuid;
use validator::Validate;

#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
#[sqlx(type_name = "status", rename_all = "snake_case")]
pub enum ActivityStatus {
    Online,
    Offline,
    Idle,
}

// todo: make as transaction with Acquire
pub async fn try_register_user<'c>(
    acq: impl Acquire<'c, Database = Postgres>,
    login: &str,
    password: SecretString,
    nickname: &str,
) -> Result<Uuid, AuthError> {
    let mut transaction = acq.begin().await?;

    let user = query!(
        r#"
            select id from credentials where login = $1
        "#,
        login
    )
    .fetch_optional(&mut transaction)
    .await?;

    if user.is_some() {
        return Err(AuthError::UserAlreadyExists);
    }

    if login.trim().is_empty() || password.expose_secret().trim().is_empty() {
        return Err(AuthError::MissingCredential);
    }

    let _ = RegisterCredentials::new(login, password.expose_secret(), &nickname)
        .validate()
        .map_err(AuthError::InvalidUsername)?;

    if !additions::pass_is_strong(password.expose_secret(), &[&login]) {
        return Err(AuthError::WeakPassword);
    }

    let hashed_pass = additions::hash_pass(password)
        .context("Failed to hash password with argon2")
        .map_err(AuthError::Unexpected)?;

    let mut nickname = nickname.trim();
    if nickname.is_empty() {
        // TODO: Generate random nickname
        nickname = "I am definitely not a chad"
    }

    // ! should be inserted at once
    let user_id = query!(
        r#"
            insert into users (nickname, activity_status)
            values ($1, $2)
            returning (id)
        "#,
        nickname,
        ActivityStatus::Online as ActivityStatus,
    )
    .fetch_one(&mut transaction)
    .await?
    .id;

    query!(
        r#"
            insert into credentials (id, login, password)
            values ($1, $2, $3)
        "#,
        user_id,
        login,
        hashed_pass
    )
    .execute(&mut transaction)
    .await?;

    transaction.commit().await?;

    Ok(user_id)
}

pub async fn verify_user_credentials(
    pool: &PgPool,
    login: &str,
    password: SecretString,
) -> Result<Uuid, AuthError> {
    debug!("Verifying credentials");
    if login.trim().is_empty() || password.expose_secret().trim().is_empty() {
        return Err(AuthError::MissingCredential)?;
    }

    let res = query!(
        r#"
            select id, password from credentials
            where login = $1
        "#,
        login
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AuthError::WrongUserOrPassword)?;

    match verify_encoded(&res.password, password.expose_secret().as_bytes())
        .context("Failed to verify credentials")
        .map_err(AuthError::Unexpected)?
    {
        true => Ok(res.id),
        false => Err(AuthError::WrongUserOrPassword),
    }
}

pub async fn generate_token_cookies(
    user_id: Uuid,
    login: &str,
    ext: &TokenExtensions,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    let access_cookie = generate_jwt_in_cookie::<Claims>(user_id, login, ext).await?;

    trace!("Access JWT: {access_cookie:#?}");

    let refresh_cookie = generate_jwt_in_cookie::<RefreshClaims>(user_id, login, ext).await?;

    trace!("Refresh JWT: {refresh_cookie:#?}");

    Ok(jar.add(access_cookie).add(refresh_cookie))
}

async fn generate_jwt_in_cookie<'a, T>(
    user_id: Uuid,
    login: &str,
    ext: &TokenExtensions,
) -> Result<Cookie<'a>, AuthError>
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

pub async fn add_token_to_blacklist(pool: &PgPool, claims: &Claims) -> Result<(), AuthError> {
    let exp = OffsetDateTime::from_unix_timestamp(claims.exp as i64)
        .context("Failed to convert timestamp to date and time with the timezone")
        .map_err(AuthError::Unexpected)?;

    let _res = query!(
        r#"
            insert into jwt_blacklist (token_id, expiry)
            values ($1, $2)
        "#,
        claims.jti,
        exp,
    )
    .execute(pool)
    .await?;

    trace!("Adding token to blacklist");
    Ok(())
}
