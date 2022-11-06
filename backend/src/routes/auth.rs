use std::future::Future;

use crate::auth::{get_token_secret, login_user, try_register_user, AuthError};
use crate::models::{AuthUser, Claims};
use anyhow::Context;
use axum::{extract, http::StatusCode, Extension, Json};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{encode, EncodingKey, Header};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use sqlx::pool::PoolConnection;
use sqlx::{PgPool, Postgres};

pub async fn post_register_user(
    pool: Extension<PgPool>,
    user: extract::Json<AuthUser>,
) -> Result<(), AuthError> {
    let conn = pool
        .acquire()
        .await
        .context("Failed to establish connection")?;
    try_register_user(
        conn,
        user.login.trim(),
        SecretString::new(user.password.trim().to_string()),
    )
    .await?;
    Ok(())
}

pub async fn post_login_user(
    pool: Extension<PgPool>,
    user: extract::Json<AuthUser>,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    const ONE_HOUR_IN_SECONDS: u64 = 3600;

    let conn = pool
        .acquire()
        .await
        .context("Failed to establish connection")?;
    let user_id = login_user(
        conn,
        &user.login,
        SecretString::new(user.password.trim().to_string()),
    )
    .await?;

    let claims = Claims {
        id: user_id,
        exp: jsonwebtoken::get_current_timestamp() + ONE_HOUR_IN_SECONDS,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_token_secret().expose_secret().as_bytes()),
    )
    .context("Failed to encrypt token")?;

    let cookie = Cookie::build("jwt", token)
        .http_only(true)
        .secure(false)
        .same_site(SameSite::Strict)
        .finish();

    Ok(jar.add(cookie))
}

pub async fn protected_zone(claims: Claims) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({ "user id": claims.id })))
}
