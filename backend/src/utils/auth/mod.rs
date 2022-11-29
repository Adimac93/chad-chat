pub mod additions;
pub mod errors;
use crate::{
    configuration::get_config,
    models::{Claims, LoginCredentials},
};
use anyhow::Context;
use argon2::verify_encoded;
use errors::*;
use jsonwebtoken::{encode, EncodingKey, Header};
use secrecy::{ExposeSecret, Secret, SecretString};
use sqlx::{query, PgPool};
use time::Duration;
use tracing::info;
use uuid::Uuid;
use validator::Validate;

pub async fn try_register_user(
    pool: &PgPool,
    login: &str,
    password: SecretString,
) -> Result<(), AuthError> {
    if login.trim().is_empty() || password.expose_secret().trim().is_empty() {
        return Err(AuthError::MissingCredential);
    }

    if !additions::pass_is_strong(password.expose_secret(), &[&login]) {
        return Err(AuthError::WeakPassword);
    }

    let _ = LoginCredentials::new(login, password.expose_secret()).validate()?;

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

    let hashed_pass = additions::hash_pass(password).context("Failed to hash pass")?;

    let res = query!(
        r#"
            insert into users (login, password)
            values ($1, $2)
        "#,
        login,
        hashed_pass
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
        id: user_id,
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
