use anyhow::Context;
use argon2::verify_encoded;
use sqlx::{query, Postgres, Pool};
use tracing::info;
use uuid::Uuid;
use secrecy::{SecretString, ExposeSecret};
use crate::{errors::AuthError, auth_utils::{pass_is_strong, hash_pass}};

pub async fn try_register_user(
    pool: &Pool<Postgres>,
    login: &str,
    password: SecretString,
) -> Result<(), AuthError> {
    if login.is_empty() || password.expose_secret().is_empty() {
        return Err(AuthError::MissingCredential);
    }

    if !pass_is_strong(password.expose_secret(), &[&login]) {
        return Err(AuthError::WeakPassword)
    }

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

    let hashed_pass = hash_pass(password).context("Failed to hash pass")?;

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
    .context("Failed to create a new user");

    info!("{res:?}");
    Ok(())
}

pub async fn login_user(
    pool: &Pool<Postgres>,
    login: &str,
    password: SecretString,
) -> Result<Uuid, AuthError> {
    if login.is_empty() || password.expose_secret().is_empty() {
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

    match verify_encoded(&res.password, password.expose_secret().as_bytes()).context("Failed to verify password")? {
        true => Ok(res.id),
        false => Err(AuthError::WrongUserOrPassword),
    }
}
