use crate::utils::auth::models::*;
use crate::{app_errors::AppError, utils::auth::*, TokenExtensions};
use axum::{debug_handler, extract, http::StatusCode, Extension, Json};
use axum::{routing::post, Router};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, DecodingKey, Validation};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use sqlx::PgPool;
use time::Duration;
use tracing::debug;

pub fn router() -> Router {
    Router::new()
        .route("/register", post(post_register_user))
        .route("/login", post(post_login_user))
        .route("/validate", post(protected_zone))
        .route("/logout", post(post_user_logout))
        .route("/refresh", post(post_refresh_user_token))
}

async fn post_register_user(
    Extension(pool): Extension<PgPool>,
    Json(register_credentials): extract::Json<RegisterCredentials>,
    token_ext: TokenExtensions,
    jar: CookieJar,
) -> Result<CookieJar, AppError> {
    let user_id = try_register_user(
        &pool,
        register_credentials.login.trim(),
        SecretString::new(register_credentials.password.trim().to_string()),
        &register_credentials.nickname,
    )
    .await?;

    let login_credentials =
        LoginCredentials::new(&register_credentials.login, &register_credentials.password);
    let jar = generate_token_cookies(user_id, &login_credentials.login, &token_ext, jar).await?;

    debug!(
        "User {} ({}) registered successfully",
        user_id, &register_credentials.login
    );

    Ok(jar)
}

async fn post_login_user(
    Extension(pool): Extension<PgPool>,
    token_ext: TokenExtensions,
    Json(login_credentials): extract::Json<LoginCredentials>,
    jar: CookieJar,
) -> Result<CookieJar, AppError> {
    // returns if credentials are wrong
    let user_id = verify_user_credentials(
        &pool,
        &login_credentials.login,
        SecretString::new(login_credentials.password.clone()),
    )
    .await?;

    let jar = generate_token_cookies(user_id, &login_credentials.login, &token_ext, jar).await?;

    debug!(
        "User {} ({}) logged in successfully",
        user_id, &login_credentials.login
    );

    Ok(jar)
}

async fn protected_zone(claims: Claims) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({ "user id": claims.user_id })))
}

async fn post_user_logout(
    Extension(pool): Extension<PgPool>,
    Extension(token_extensions): Extension<TokenExtensions>,
    jar: CookieJar,
) -> Result<CookieJar, AppError> {
    let mut validation = Validation::default();
    validation.leeway = 5;

    if let Some(access_token_cookie) = jar.get("jwt") {
        let data = decode::<Claims>(
            access_token_cookie.value(),
            &DecodingKey::from_secret(token_extensions.access.0.expose_secret().as_bytes()),
            &validation,
        );

        if let Ok(token_data) = data {
            let _ = &token_data.claims.add_token_to_blacklist(&pool).await?;
        }
    };

    if let Some(refresh_token_cookie) = jar.get("refresh-jwt") {
        let data = decode::<RefreshClaims>(
            refresh_token_cookie.value(),
            &DecodingKey::from_secret(token_extensions.access.0.expose_secret().as_bytes()),
            &validation,
        );

        if let Ok(token_data) = data {
            let _ = &token_data.claims.add_token_to_blacklist(&pool).await?;
        }
    };

    debug!("User logged out successfully");

    Ok(jar
        .remove(remove_cookie("jwt"))
        .remove(remove_cookie("refresh-jwt")))
}

fn remove_cookie(name: &str) -> Cookie {
    Cookie::build(name, "")
        .path("/")
        .max_age(Duration::seconds(0))
        .finish()
}

#[debug_handler]
async fn post_refresh_user_token(
    Extension(pool): Extension<PgPool>,
    ext: TokenExtensions,
    refresh_claims: RefreshClaims,
    jar: CookieJar,
) -> Result<CookieJar, AppError> {
    let jar =
        generate_token_cookies(refresh_claims.user_id, &refresh_claims.login, &ext, jar).await?;

    refresh_claims.add_token_to_blacklist(&pool).await?;

    debug!(
        "User {} ({})'s access token refreshed successfully",
        &refresh_claims.user_id, &refresh_claims.login
    );

    Ok(jar)
}
