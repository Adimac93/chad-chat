use crate::{
    models::{Claims, LoginCredentials},
    utils::auth::{errors::AuthError, *},
    JwtSecret,
};
use axum::{extract, http::StatusCode, Extension, Json};
use axum::{routing::post, Router};
use axum_extra::extract::cookie::{Cookie, SameSite};
use axum_extra::extract::CookieJar;
use secrecy::SecretString;
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
    user: extract::Json<LoginCredentials>,
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
    Extension(JwtSecret(jwt_key)): Extension<JwtSecret>,
    Json(user): extract::Json<LoginCredentials>,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    let token = authorize_user(&pool, user, Duration::hours(2), jwt_key).await?;
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
