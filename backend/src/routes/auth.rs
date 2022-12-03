use crate::{
    models::{Claims, LoginCredentials, RefreshClaims, RegisterCredentials},
    utils::auth::{errors::AuthError, *},
    JwtSecret, RefreshJwtSecret,
};
use axum::{extract, http::StatusCode, routing::get, Extension, Json};
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
        .route("/user-validation", post(protected_zone))
        .route("/logout", post(post_user_logout))
        .route("/refresh-token", post(post_refresh_user_token))
}

async fn post_register_user(
    Extension(pool): Extension<PgPool>,
    user: extract::Json<RegisterCredentials>,
) -> Result<(), AuthError> {
    try_register_user(
        &pool,
        user.login.trim(),
        SecretString::new(user.password.trim().to_string()),
        &user.nickname,
    )
    .await
}

async fn post_login_user(
    Extension(pool): Extension<PgPool>,
    Extension(JwtSecret(jwt_key)): Extension<JwtSecret>,
    Extension(RefreshJwtSecret(refresh_jwt_key)): Extension<RefreshJwtSecret>,
    Json(user): extract::Json<LoginCredentials>,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    // returns if credentials are wrong
    let user_id = login_user(&pool, &user.login, SecretString::new(user.password)).await?;

    let access_token =
        generate_jwt_token(user_id, &user.login, Duration::minutes(10), &jwt_key).await?;
    let access_cookie = generate_cookie(access_token, JwtTokenType::Access).await;

    let refresh_token =
        generate_refresh_jwt_token(user_id, &user.login, Duration::days(7), &refresh_jwt_key)
            .await?;
    let refresh_cookie = generate_cookie(refresh_token, JwtTokenType::Refresh).await;

    let jar = jar.add(access_cookie);
    Ok(jar.add(refresh_cookie))
}

async fn protected_zone(claims: Claims) -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({ "user id": claims.user_id })))
}

async fn post_user_logout(
    Extension(pool): Extension<PgPool>,
    Extension(RefreshJwtSecret(refresh_jwt_key)): Extension<RefreshJwtSecret>,
    Extension(JwtSecret(jwt_key)): Extension<JwtSecret>,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    let mut validation = Validation::default();
    validation.leeway = 5;

    if let Some(access_token_cookie) = jar.get("jwt") {
        let data = decode::<Claims>(
            access_token_cookie.value(),
            &DecodingKey::from_secret(jwt_key.expose_secret().as_bytes()),
            &validation,
        );

        if let Ok(token_data) = data {
            add_token_to_blacklist(&pool, &token_data.claims).await?;
        }
    };

    if let Some(refresh_token_cookie) = jar.get("refresh-jwt") {
        let data = decode::<Claims>(
            refresh_token_cookie.value(),
            &DecodingKey::from_secret(refresh_jwt_key.expose_secret().as_bytes()),
            &validation,
        );

        if let Ok(token_data) = data {
            add_token_to_blacklist(&pool, &token_data.claims).await?;
        }
    };

    debug!("Removing client cookies");
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

async fn post_refresh_user_token(
    Extension(JwtSecret(jwt_key)): Extension<JwtSecret>,
    refresh_claims: RefreshClaims,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    let access_token = generate_jwt_token(
        refresh_claims.user_id,
        &refresh_claims.login,
        Duration::minutes(10),
        &jwt_key,
    )
    .await?;

    let cookie = generate_cookie(access_token, JwtTokenType::Access).await;

    Ok(jar.add(cookie))
}
