use crate::errors::AppError;
use crate::modules::extractors::addr::ClientAddr;
use crate::modules::extractors::jwt::TokenExtractors;
use crate::modules::smtp::Mailer;
use crate::state::{AppState, RdPool};
use crate::utils::auth::models::*;
use crate::utils::auth::*;
use crate::utils::chat::get_user_email_by_id;
use anyhow::Context;
use axum::extract::{ConnectInfo, State};

use axum::{debug_handler, extract, http::StatusCode, Json};
use axum::{
    routing::post,
    Router,
};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use jsonwebtoken::Validation;
use redis::{transaction, pipe, ConnectionLike};
use secrecy::SecretString;
use serde_json::{json, Value};
use sqlx::PgPool;
use time::Duration;
use tracing::debug;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(post_register_user))
        .route("/login", post(post_login_user))
        .route("/validate", post(protected_zone))
        .route("/logout", post(post_user_logout))
        .route("/refresh", post(post_refresh_user_token))
}

#[debug_handler(state = AppState)]
async fn post_register_user(
    State(pgpool): State<PgPool>,
    State(mailer): State<Mailer>,
    State(token_ext): State<TokenExtractors>,
    ConnectInfo(addr): ConnectInfo<ClientAddr>,
    jar: CookieJar,
    Json(register_credentials): extract::Json<RegisterCredentials>,
) -> Result<CookieJar, AppError> {
    let user_id = try_register_user(
        &pgpool,
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
        "User {} registered successfully",
        user_id
    );

    Ok(jar)
}

#[debug_handler(state = AppState)]
async fn post_login_user(
    State(pool): State<PgPool>,
    State(token_ext): State<TokenExtractors>,
    ConnectInfo(_addr): ConnectInfo<ClientAddr>,
    jar: CookieJar,
    Json(login_credentials): extract::Json<LoginCredentials>,
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
        "User {} logged in successfully",
        user_id
    );

    Ok(jar)
}

async fn protected_zone(claims: Claims) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({ "user id": claims.user_id })))
}

async fn post_user_logout(
    State(mut pool): State<RdPool>,
    State(token_extensions): State<TokenExtractors>,
    jar: CookieJar,
) -> Result<CookieJar, AppError> {
    let mut validation = Validation::default();
    validation.leeway = 5;

    // let mut pg_tr = pool.begin().await?;

    let mut pipe = pipe();
    let atomic_pipe = pipe.atomic();

    if let Some(access_token_cookie) = jar.get("jwt") {
        let verify_res = validate_access_token(access_token_cookie, &token_extensions.access.0);

        if let Ok(claims) = verify_res {
            add_access_token_to_blacklist(atomic_pipe, claims).await?;
        }
    };

    if let Some(refresh_token_cookie) = jar.get("refresh-jwt") {
        let verify_res = validate_refresh_token(refresh_token_cookie, &token_extensions.refresh.0);

        if let Ok(claims) = verify_res {
            add_refresh_token_to_blacklist(atomic_pipe, claims).await?;
        }
    };

    atomic_pipe.query_async(&mut pool).await.context("Failed to add tokens to the blacklist")?;

    // pg_tr.commit().await?;

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

// #[debug_handler]
async fn post_refresh_user_token(
    refresh_claims: RefreshClaims,
    State(pool): State<PgPool>,
    State(mut rdpool): State<RdPool>,
    State(ext): State<TokenExtractors>,
    jar: CookieJar,
) -> Result<CookieJar, AppError> {
    let user_id = refresh_claims.user_id;

    let email = get_user_email_by_id(&pool, &user_id).await?;
    let access_token_cookie = create_access_token(user_id, email, &ext.access).await?;
    // add_refresh_token_to_blacklist(&mut rdpool, refresh_claims).await?;

    debug!(
        "User {} access token refreshed successfully",
        user_id
    );

    Ok(jar.add(access_token_cookie))
}
