pub mod additions;
pub mod errors;
use crate::{
    configuration::get_config,
    models::{Claims, LoginCredentials, RegisterCredentials, RefreshClaims},
};
use anyhow::{Context, Error};
use argon2::verify_encoded;
use axum_extra::extract::cookie::{Cookie, SameSite};
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

// pub async fn authorize_user(
//     pool: &PgPool,
//     user: LoginCredentials,
//     duration: Duration,
//     key: Secret<String>,
// ) -> Result<String, AuthError> {
//     let user_id = login_user(
//         &pool,
//         &user.login.trim(),
//         SecretString::new(user.password.trim().to_string()),
//     )
//     .await?;

//     let token = generate_jwt_token(user_id, &user.login, duration, &key).await?;

//     Ok(token)
// }

pub async fn generate_jwt_token(
    user_id: Uuid,
    login: &str,
    duration: Duration,
    key: &Secret<String>
) -> Result<String, Error> {
    let claims = Claims::new(user_id, login, duration);

    // let _config = get_config().expect("Failed to read configuration");
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
    let claims = RefreshClaims::new(user_id, login, duration);

    // let _config = get_config().expect("Failed to read configuration");
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

pub async fn generate_cookie<'a>(token: String, token_type: JwtTokenType) -> Cookie<'a> {
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
