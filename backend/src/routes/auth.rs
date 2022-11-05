use anyhow::Context;
use axum::{Extension, extract, http::StatusCode, Json};
use jsonwebtoken::{encode, Header, EncodingKey};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{Value, json};
use sqlx::PgPool;
use crate::models::{AuthUser, Claims};
use crate::auth::{try_register_user, login_user, get_token_secret, AuthError};

pub async fn post_register_user(
    pool: Extension<PgPool>,
    user: extract::Json<AuthUser>,
) -> Result<(), AuthError> {
    let conn = pool.acquire().await.context("Failed to establish connection")?;
    try_register_user(conn, user.login.trim(), SecretString::new(user.password.trim().to_string())).await?;
    Ok(())
}

pub async fn post_login_user(
    pool: Extension<PgPool>,
    user: extract::Json<AuthUser>,
) -> Result<Json<Value>, AuthError> {
    const ONE_HOUR_IN_SECONDS: u64 = 3600;

    let conn = pool.acquire().await.context("Failed to establish connection")?;
    let user_id = login_user(conn, &user.login, SecretString::new(user.password.trim().to_string())).await?;

    let claims = Claims {
        id: user_id,
        exp: jsonwebtoken::get_current_timestamp() + ONE_HOUR_IN_SECONDS,
    };

    let token = encode(&Header::default(), &claims, &EncodingKey::from_secret(get_token_secret().expose_secret().as_bytes()))
        .context("Failed to encrypt token")?;

    Ok(Json(json!({ "access_token": token, "type": "Bearer" })))
}

pub async fn protected_zone(claims: Claims) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({ "user id": claims.id })))
}