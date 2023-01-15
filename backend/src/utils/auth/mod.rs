pub mod additions;
pub mod errors;
pub mod models;
pub mod tokens;

use crate::{database::RdPool, TokenExtensions};
use anyhow::Context;
use argon2::verify_encoded;
use axum_extra::extract::{cookie::Cookie, CookieJar};
use errors::*;
use models::*;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use sqlx::{query, Acquire, PgPool, Postgres};
use time::OffsetDateTime;
use tracing::{debug, trace};
use uuid::Uuid;
use validator::Validate;

use self::{additions::random_username_tag, tokens::Token};

use super::email::Mailer;

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
    rdpool: &mut RdPool,
    mailer: Option<Mailer>,
    email: &str,
    password: SecretString,
    username: &str,
) -> Result<Uuid, AuthError> {
    let mut transaction = pool.begin().await?;

    let user = query!(
        r#"
            select id from credentials where email = $1
        "#,
        email
    )
    .fetch_optional(&mut transaction)
    .await?;

    if user.is_some() {
        return Err(AuthError::UserAlreadyExists);
    }

    if email.trim().is_empty() || password.expose_secret().trim().is_empty() {
        return Err(AuthError::MissingCredential);
    }

    let _ = RegisterCredentials::new(email, password.expose_secret(), &username)
        .validate()
        .map_err(AuthError::InvalidEmail)?;

    if !additions::pass_is_strong(password.expose_secret(), &[&email]) {
        return Err(AuthError::WeakPassword);
    }

    let hashed_pass = additions::hash_pass(password)
        .context("Failed to hash password with argon2")
        .map_err(AuthError::Unexpected)?;

    let mut username = username.trim();
    if username.is_empty() {
        // TODO: Generate random username
        username = "I am definitely not a chad"
    }

    let used_tags = query!(
        r#"
            select tag from users
            where username = $1
        "#,
        username
    )
    .fetch_all(&mut transaction)
    .await?;

    let tag = random_username_tag(used_tags.into_iter().map(|record| record.tag).collect())
        .ok_or(AuthError::TagOverflow)?;

    let user_id = query!(
        r#"
            insert into users (username, tag, activity_status)
            values ($1, $2, $3)
            returning (id)
        "#,
        username,
        tag,
        ActivityStatus::Online as ActivityStatus,
    )
    .fetch_one(&mut transaction)
    .await?
    .id;

    query!(
        r#"
            insert into credentials (id, email, password)
            values ($1, $2, $3)
        "#,
        user_id,
        email,
        hashed_pass
    )
    .execute(&mut transaction)
    .await?;

    transaction.commit().await?;

    if let Some(mailer) = mailer {
        let token_id = Token::Registration
            .gen_token_with_duration(rdpool, &user_id)
            .await?;
        mailer.send_verification(email, &token_id).await?;
    }

    Ok(user_id)
}

pub async fn verify_user_credentials(
    pool: &PgPool,
    email: &str,
    password: SecretString,
) -> Result<Uuid, AuthError> {
    debug!("Verifying credentials");
    if email.trim().is_empty() || password.expose_secret().trim().is_empty() {
        return Err(AuthError::MissingCredential)?;
    }

    let res = query!(
        r#"
            select id, password from credentials
            where email = $1
        "#,
        email
    )
    .fetch_optional(pool)
    .await?
    .ok_or(AuthError::WrongEmailOrPassword)?;

    match verify_encoded(&res.password, password.expose_secret().as_bytes())
        .context("Failed to verify credentials")
        .map_err(AuthError::Unexpected)?
    {
        true => Ok(res.id),
        false => Err(AuthError::WrongEmailOrPassword),
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

pub async fn use_reg_token(rdpool: &mut RdPool, token_id: &Uuid) -> Result<(), AuthError> {
    Token::Registration.use_token(rdpool, token_id).await?;
    // todo: change status to verified
    Ok(())
}
