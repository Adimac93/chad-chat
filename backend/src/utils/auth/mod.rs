pub mod additions;
pub mod errors;
use crate::{
    models::{AuthToken, Claims, RefreshClaims, RegisterCredentials}, TokenExtensions,
};
use anyhow::Context;
use argon2::verify_encoded;
use axum_extra::extract::{CookieJar, cookie::Cookie};
use errors::*;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{query, PgPool};
use time::OffsetDateTime;
use tracing::{debug, trace};
use uuid::Uuid;
use validator::Validate;

#[derive(sqlx::Type, Debug)]
#[sqlx(type_name = "status", rename_all = "snake_case")]
pub enum ActivityStatus {
    Online,
    Offline,
    Idle,
}

pub async fn try_register_user(
    pool: &PgPool,
    login: &str,
    password: SecretString,
    nickname: &str,
) -> Result<Uuid, AuthError> {
    let user = query!(
        r#"
            select id from users where login = $1
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
            insert into users (login, password, nickname, activity_status)
            values ($1, $2, $3, $4)
            returning (id)
        "#,
        login,
        hashed_pass,
        nickname,
        ActivityStatus::Online as ActivityStatus,
    )
    .fetch_one(pool)
    .await
    .context("Failed to create a new user")?
    .id;

    Ok(user_id)
}

pub async fn verify_user_credentials(
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
            select id, password from users where login = $1
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

pub async fn generate_token_cookies (
    user_id: Uuid,
    login: &str,
    ext: &TokenExtensions,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    let access_cookie = generate_jwt_in_cookie::<Claims> (user_id, login, ext).await?;

    trace!("Access JWT: {access_cookie:#?}");

    let refresh_cookie = generate_jwt_in_cookie::<RefreshClaims> (user_id, login, ext).await?;

    trace!("Refresh JWT: {refresh_cookie:#?}");

    Ok(jar.add(access_cookie).add(refresh_cookie))
}

async fn generate_jwt_in_cookie<'a, T>(
    user_id: Uuid,
    login: &str,
    ext: &TokenExtensions,
) -> Result<Cookie<'a>, AuthError>
where T: AuthToken {
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

    trace!("Adding token to blacklist");
    Ok(())
}
