use crate::{
    auth::{login_user, try_register_user},
    auth_utils::get_token_secret,
    errors::AuthError,
    models::{AuthUser, Claims},
};
use anyhow::Context;
use axum::{extract, http::StatusCode, Extension, Json};
use axum::{
    response::Html,
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{encode, EncodingKey, Header};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use sqlx::PgPool;

pub fn router() -> Router {
    Router::new()
        .route("/register", get(register_index).post(post_register_user))
        .route("/login", get(login_index).post(post_login_user))
        .route("/user-validation", post(protected_zone))
}

pub async fn post_register_user(
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

pub async fn post_login_user(
    Extension(pool): Extension<PgPool>,
    user: extract::Json<AuthUser>,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    const ONE_HOUR_IN_SECONDS: u64 = 3600;
    let user_id = login_user(
        &pool,
        &user.login,
        SecretString::new(user.password.trim().to_string()),
    )
    .await?;

    let claims = Claims {
        id: user_id,
        login: user.login.clone(),
        exp: jsonwebtoken::get_current_timestamp() + ONE_HOUR_IN_SECONDS,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(get_token_secret().expose_secret().as_bytes()),
    )
    .context("Failed to encrypt token")?;

    let cookie = Cookie::build("jwt", token)
        .http_only(false)
        .secure(false)
        .same_site(SameSite::Strict)
        .path("/")
        .finish();

    Ok(jar.add(cookie))
}

pub async fn protected_zone(claims: Claims) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({ "user id": claims.id })))
}

pub async fn login_index() -> Html<&'static str> {
    Html(std::include_str!("../../login.html"))
}

pub async fn register_index() -> Html<&'static str> {
    Html(std::include_str!("../../register.html"))
}
