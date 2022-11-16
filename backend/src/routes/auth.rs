﻿use crate::{
    models::{AuthUser, Claims},
    utils::auth::{errors::AuthError, *},
};
use axum::{extract, http::StatusCode, Extension, Json};
use axum::{
    response::Html,
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use sqlx::PgPool;
use time::Duration;

pub fn router() -> Router {
    Router::new()
        .route("/register", post(post_register_user))
        .route("/login", post(post_login_user))
        .route("/user-validation", post(protected_zone))
}

async fn post_register_user(
    Extension(pool): Extension<PgPool>,
    user: extract::Json<AuthUser>,
) -> Result<(), AuthError> {
    try_register_user(
        &pool,
        user.login.trim(),
        SecretString::new(user.password.trim().to_string()),
    )
    .await
}

async fn post_login_user(
    Extension(pool): Extension<PgPool>,
    Json(user): extract::Json<AuthUser>,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    let token = authorize_user(&pool, user, Duration::hours(2)).await?;
    let cookie = Cookie::build("jwt", token)
        .http_only(false)
        .secure(true)
        .same_site(SameSite::None)
        .path("/")
        .finish();

    Ok(jar.add(cookie))
}

async fn protected_zone(claims: Claims) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({ "user id": claims.id })))
}
