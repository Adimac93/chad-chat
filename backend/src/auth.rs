﻿use anyhow::Context;
use argon2::{verify_encoded, hash_encoded};
use axum::{response::IntoResponse, http::StatusCode, Json};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde_json::json;
use sqlx::{query, pool::PoolConnection, Postgres};
use thiserror::Error;
use tracing::info;
use uuid::Uuid;
use secrecy::{SecretString, ExposeSecret};

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Missing credential")]
    MissingCredential,
    #[error("Password is too weak")]
    WeakPassword,
    #[error("Incorrect user or password")]
    WrongUserOrPassword,
    #[error("Invalid or expired token")]
    InvalidToken,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error)
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let status_code = match &self {
            AuthError::UserAlreadyExists => StatusCode::BAD_REQUEST,
            AuthError::MissingCredential => StatusCode::BAD_REQUEST,
            AuthError::WeakPassword => StatusCode::BAD_REQUEST,
            AuthError::WrongUserOrPassword => StatusCode::UNAUTHORIZED,
            AuthError::InvalidToken => StatusCode::UNAUTHORIZED,
            AuthError::Unexpected(e) => {
                tracing::error!("Internal server error: {e:?}");
                StatusCode::INTERNAL_SERVER_ERROR
            },
        };
        
        let info = match self {
            AuthError::Unexpected(_) => "Unexpected server error".into(),
            _ => format!("{self:?}")
        };

        (status_code, Json(json!({ "error_info": info }))).into_response()
    }
}

pub async fn try_register_user(
    mut conn: PoolConnection<Postgres>,
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
    .fetch_optional(&mut conn)
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
    .execute(&mut conn)
    .await
    .context("Failed to create a new user");

    info!("{res:?}");
    Ok(())
}

pub async fn login_user(
    mut conn: PoolConnection<Postgres>,
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
    .fetch_optional(&mut conn)
    .await
    .context("Failed to select user by login")?
    .ok_or(AuthError::WrongUserOrPassword)?;

    if verify_encoded(&res.password, password.expose_secret().as_bytes()).context("Failed to verify password")? {
        Ok(res.id)
    } else {
        Err(AuthError::WrongUserOrPassword)
    }
}

fn hash_pass(pass: SecretString) -> Result<String, AuthError> {
    Ok(hash_encoded(pass.expose_secret().as_bytes(), random_salt().as_bytes(), &argon2::Config::default()).context("Failed to hash pass")?)
}

fn random_salt() -> String {
    let mut rng = thread_rng();
    (0..8).map(|_| rng.sample(Alphanumeric) as char).collect()
}

fn pass_is_strong(user_password: &str, user_inputs: &[&str]) -> bool {
    let score = zxcvbn::zxcvbn(user_password, user_inputs);
    match score {
        Ok(s) => s.score() >= 3,
        Err(_) => false,
    }
}

pub fn get_token_secret() -> SecretString {
    SecretString::new(std::env::var("TOKEN_SECRET").expect("Cannot find token secret"))
}