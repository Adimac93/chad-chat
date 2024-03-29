﻿use crate::modules::database::RdPool;
use crate::modules::extractors::addr::ClientAddr;
use crate::modules::smtp::Mailer;
use crate::utils::auth::models::*;
use crate::{app_errors::AppError, utils::auth::*, TokenExtractors};
use axum::extract::{ConnectInfo, Path};
use axum::response::IntoResponse;
use axum::{debug_handler, extract, http::StatusCode, Extension, Json};
use axum::{
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, DecodingKey, Validation};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use sqlx::PgPool;
use time::Duration;
use tracing::debug;
use uuid::Uuid;
pub fn router() -> Router {
    Router::new()
        .route("/register", post(post_register_user))
        .route("/login", post(post_login_user))
        .route("/validate", post(protected_zone))
        .route("/logout", post(post_user_logout))
        .route("/refresh", post(post_refresh_user_token))
        .route("/verify/registration/:token_id", get(verify_token))
}

async fn post_register_user(
    Extension(pgpool): Extension<PgPool>,
    Extension(mut rdpool): Extension<RdPool>,
    Extension(mailer): Extension<Mailer>,
    ConnectInfo(addr): ConnectInfo<ClientAddr>,
    Json(register_credentials): extract::Json<RegisterCredentials>,
    token_ext: TokenExtractors,
    jar: CookieJar,
) -> Result<CookieJar, AppError> {
    let user_id = try_register_user(
        &pgpool,
        &mut rdpool,
        addr.network(),
        Some(mailer),
        register_credentials.email.trim(),
        SecretString::new(register_credentials.password.trim().to_string()),
        &register_credentials.username,
    )
    .await?;

    let login_credentials =
        LoginCredentials::new(&register_credentials.email, &register_credentials.password);
    let jar = generate_token_cookies(user_id, &login_credentials.email, &token_ext, jar).await?;

    debug!(
        "User {} ({}) registered successfully",
        user_id, &register_credentials.email
    );

    Ok(jar)
}

async fn post_login_user(
    Extension(pool): Extension<PgPool>,
    token_ext: TokenExtractors,
    ConnectInfo(addr): ConnectInfo<ClientAddr>,
    Json(login_credentials): extract::Json<LoginCredentials>,
    jar: CookieJar,
) -> Result<CookieJar, AppError> {
    // returns if credentials are wrong
    let user_id = verify_user_credentials(
        &pool,
        &login_credentials.email,
        SecretString::new(login_credentials.password.clone()),
    )
    .await?;

    let jar = generate_token_cookies(user_id, &login_credentials.email, &token_ext, jar).await?;

    debug!(
        "User {} ({}) logged in successfully",
        user_id, &login_credentials.email
    );

    Ok(jar)
}

async fn protected_zone(claims: Claims) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({ "user id": claims.user_id })))
}

async fn post_user_logout(
    Extension(pool): Extension<PgPool>,
    Extension(token_extensions): Extension<TokenExtractors>,
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
    ext: TokenExtractors,
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

async fn verify_token(
    Extension(mut rdpool): Extension<RdPool>,
    Path(token_id): Path<Uuid>,
) -> Result<(), AppError> {
    Ok(use_reg_token(&mut rdpool, &token_id).await?)
}
