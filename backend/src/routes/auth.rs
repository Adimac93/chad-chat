use crate::{
    models::{Claims, LoginCredentials, RefreshClaims, RegisterCredentials},
    utils::auth::{errors::AuthError, *},
    JwtSecret, RefreshJwtSecret,
};
use axum::{extract, http::StatusCode, Extension, Json};
use axum::{routing::post, Router};
use axum_extra::extract::cookie::Cookie;
use axum_extra::extract::CookieJar;
use jsonwebtoken::{decode, DecodingKey, Validation};
use secrecy::{ExposeSecret, SecretString};
use serde_json::{json, Value};
use sqlx::PgPool;
use time::Duration;
use tracing::debug;

pub const JWT_ACCESS_TOKEN_EXPIRATION: Duration = Duration::minutes(5);
pub const JWT_REFRESH_TOKEN_EXPIRATION: Duration = Duration::days(7);

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
    register_credentials: extract::Json<RegisterCredentials>,
    Extension(RefreshJwtSecret(refresh_jwt_key)): Extension<RefreshJwtSecret>,
    Extension(JwtSecret(jwt_key)): Extension<JwtSecret>,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    let user_id = try_register_user(
        &pool,
        register_credentials.login.trim(),
        SecretString::new(register_credentials.password.trim().to_string()),
        &register_credentials.nickname,
    )
    .await?;

    let login_credentials = LoginCredentials::new(&register_credentials.login, &register_credentials.password);
    let jar = login_user(user_id, &login_credentials, jwt_key, refresh_jwt_key, jar).await?;

    debug!("User {} ({}) registered successfully", user_id, &register_credentials.login);

    Ok(jar)
}

async fn post_login_user(
    Extension(pool): Extension<PgPool>,
    Extension(JwtSecret(jwt_key)): Extension<JwtSecret>,
    Extension(RefreshJwtSecret(refresh_jwt_key)): Extension<RefreshJwtSecret>,
    Json(login_credentials): extract::Json<LoginCredentials>,
    jar: CookieJar,
) -> Result<CookieJar, AuthError> {
    // returns if credentials are wrong
    let user_id = verify_user_credentials(&pool, &login_credentials.login, SecretString::new(login_credentials.password.clone())).await?;

    let jar = login_user(user_id, &login_credentials, jwt_key, refresh_jwt_key, jar).await?;

    debug!("User {} ({}) logged in successfully", user_id, &login_credentials.login);

    Ok(jar)
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

    debug!("User logged out successfully");
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
        JWT_ACCESS_TOKEN_EXPIRATION,
        &jwt_key,
    )
    .await?;

    debug!("Generating a cookie");
    let cookie = generate_cookie(access_token, JwtTokenType::Access).await;

    debug!("User {} ({})'s access token refreshed successfully", &refresh_claims.user_id, &refresh_claims.login);

    Ok(jar.add(cookie))
}
