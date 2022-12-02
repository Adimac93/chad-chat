pub mod additions;
pub mod errors;
use crate::{
    configuration::get_config,
    models::{Claims, LoginCredentials, RegisterCredentials},
};
use anyhow::Context;
use argon2::verify_encoded;
use errors::*;
use jsonwebtoken::{encode, EncodingKey, Header};
use secrecy::{ExposeSecret, Secret, SecretString};
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
) -> Result<(), AuthError> {
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

    let res = query!(
        r#"
            insert into users (login, password, nickname)
            values ($1, $2, $3)
        "#,
        login,
        hashed_pass,
        nickname
    )
    .execute(pool)
    .await
    .context("Failed to create a new user")?;

    info!("{res:?}");
    Ok(())
}

pub async fn login_user(
    pool: &PgPool,
    login: &str,
    password: SecretString,
) -> Result<Uuid, AuthError> {
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

pub async fn authorize_user(
    pool: &PgPool,
    user: LoginCredentials,
    duration: Duration,
    key: Secret<String>,
) -> Result<String, AuthError> {
    let user_id = login_user(
        &pool,
        &user.login.trim(),
        SecretString::new(user.password.trim().to_string()),
    )
    .await?;

    let claims = Claims {
        jti: Uuid::new_v4(),
        user_id,
        login: user.login.clone(),
        exp: jsonwebtoken::get_current_timestamp() + duration.whole_seconds().abs() as u64,
    };

    let _config = get_config().expect("Failed to read configuration");
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(key.expose_secret().as_bytes()),
    )
    .context("Failed to encrypt token")?;

    Ok(token)
}

pub async fn add_token_to_blacklist(pool: &PgPool, claims: Claims) -> Result<(), AuthError> {
    let exp = OffsetDateTime::from_unix_timestamp(claims.exp as i64)
        .context("Failed to convert timestamp to date and time with the timezone")?;

    let res = query!(
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