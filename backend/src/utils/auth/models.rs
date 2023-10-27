use crate::{
    errors::AppError,
    modules::extractors::jwt::{JwtAccessSecret, JwtRefreshSecret, TokenExtractors},
    state::AppState,
};
use anyhow::Context;
use axum::{
    async_trait,
    extract::{self, FromRequest, FromRequestParts},
    http::request::Parts,
};
use axum_extra::extract::{
    cookie::{Cookie, SameSite},
    CookieJar,
};
use hyper::StatusCode;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;
use validator::Validate;

#[async_trait]
pub trait AuthToken {
    const JWT_EXPIRATION: Duration;

    async fn generate_cookie<'a>(token: String) -> Cookie<'a>;
    async fn generate_jwt(
        user_id: Uuid,
        login: &str,
        duration: Duration,
        key: &Secret<String>,
    ) -> Result<String, AppError>;
    async fn get_jwt_key(ext: &TokenExtractors) -> Secret<String>;
    async fn get_jwt_cookie(jar: CookieJar) -> Result<Cookie<'static>, AppError>;
    async fn decode_jwt(token: &str, key: Secret<String>) -> Result<Self, AppError>
    where
        Self: Sized;
    async fn check_if_in_blacklist(&self, pool: &PgPool) -> Result<bool, AppError>;
    async fn add_token_to_blacklist(&self, pool: &PgPool) -> Result<(), AppError>;
}

#[async_trait]
impl AuthToken for Claims {
    const JWT_EXPIRATION: Duration = Duration::seconds(15);

    async fn get_jwt_key(ext: &TokenExtractors) -> Secret<String> {
        let JwtAccessSecret(jwt_key) = ext.access.clone();

        jwt_key
    }

    async fn get_jwt_cookie(jar: CookieJar) -> Result<Cookie<'static>, AppError> {
        jar.get("jwt")
            .ok_or(AppError::exp(StatusCode::UNAUTHORIZED, "Invalid token"))
            .cloned()
    }

    async fn decode_jwt(token: &str, key: Secret<String>) -> Result<Self, AppError> {
        // decode token - validation setup
        let mut validation = Validation::default();
        validation.leeway = 5;

        // decode token - try to decode token with a provided jwt key
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(key.expose_secret().as_bytes()),
            &validation,
        )
        .map_err(|_e| AppError::exp(StatusCode::UNAUTHORIZED, "Invalid token"))?;

        Ok(data.claims)
    }

    async fn check_if_in_blacklist(&self, pool: &PgPool) -> Result<bool, AppError> {
        // verify blacklist
        Ok(query!(
            r#"
                SELECT * FROM jwt_blacklist
                WHERE token_id = $1;
            "#,
            self.jti
        )
        .fetch_optional(pool)
        .await
        .context("Failed to verify token with the blacklist")?
        .is_some())
    }

    async fn generate_cookie<'a>(token: String) -> Cookie<'a> {
        Cookie::build(String::from("jwt"), token)
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .path("/")
            .finish()
    }

    async fn generate_jwt(
        user_id: Uuid,
        login: &str,
        duration: Duration,
        key: &Secret<String>,
    ) -> Result<String, AppError> {
        let claims = Claims::new(user_id, login, duration);

        Ok(encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(key.expose_secret().as_bytes()),
        )
        .context("Failed to encrypt token")?)
    }

    async fn add_token_to_blacklist(&self, pool: &PgPool) -> Result<(), AppError> {
        let exp = OffsetDateTime::from_unix_timestamp(self.exp as i64)
            .context("Failed to convert timestamp to date and time with the timezone")?;

        let _res = query!(
            r#"
                INSERT INTO jwt_blacklist (token_id, expiry)
                VALUES ($1, $2)
            "#,
            self.jti,
            exp,
        )
        .execute(pool)
        .await
        .context("Failed to add token to the blacklist")?;

        Ok(())
    }
}

#[async_trait]
impl AuthToken for RefreshClaims {
    const JWT_EXPIRATION: Duration = Duration::days(7);

    async fn get_jwt_key(ext: &TokenExtractors) -> Secret<String> {
        let JwtRefreshSecret(jwt_key) = ext.refresh.clone();

        jwt_key
    }

    async fn get_jwt_cookie(jar: CookieJar) -> Result<Cookie<'static>, AppError> {
        jar.get("refresh-jwt")
            .ok_or(AppError::exp(StatusCode::UNAUTHORIZED, "Invalid token"))
            .cloned()
    }

    async fn decode_jwt(token: &str, key: Secret<String>) -> Result<Self, AppError> {
        // decode token - validation setup
        let mut validation = Validation::default();
        validation.leeway = 5;

        // decode token - try to decode token with a provided jwt key
        let data = decode::<RefreshClaims>(
            token,
            &DecodingKey::from_secret(key.expose_secret().as_bytes()),
            &validation,
        )
        .map_err(|_e| AppError::exp(StatusCode::UNAUTHORIZED, "Invalid token"))?;

        Ok(data.claims)
    }

    async fn check_if_in_blacklist(&self, pool: &PgPool) -> Result<bool, AppError> {
        // verify blacklist
        Ok(query!(
            r#"
                SELECT * FROM jwt_blacklist
                WHERE token_id = $1;
            "#,
            self.jti
        )
        .fetch_optional(pool)
        .await
        .context("Failed to verify token with the blacklist")?
        .is_some())
    }

    async fn generate_cookie<'a>(token: String) -> Cookie<'a> {
        Cookie::build(String::from("refresh-jwt"), token)
            .http_only(true)
            .secure(true)
            .same_site(SameSite::Strict)
            .path("/")
            .finish()
    }

    async fn generate_jwt(
        user_id: Uuid,
        login: &str,
        duration: Duration,
        key: &Secret<String>,
    ) -> Result<String, AppError> {
        let refresh_claims = RefreshClaims::new(user_id, login, duration);

        Ok(encode(
            &Header::default(),
            &refresh_claims,
            &EncodingKey::from_secret(key.expose_secret().as_bytes()),
        )
        .context("Failed to encrypt token")?)
    }

    async fn add_token_to_blacklist(&self, pool: &PgPool) -> Result<(), AppError> {
        let exp = OffsetDateTime::from_unix_timestamp(self.exp as i64)
            .context("Failed to convert timestamp to date and time with the timezone")?;

        let _res = query!(
            r#"
                INSERT INTO jwt_blacklist (token_id, expiry)
                VALUES ($1, $2)
            "#,
            self.jti,
            exp,
        )
        .execute(pool)
        .await
        .context("Failed to add token to the blacklist")?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Claims {
    pub jti: Uuid,
    pub user_id: Uuid,
    pub login: String,
    pub exp: u64,
}

impl Claims {
    pub fn new(user_id: Uuid, login: &str, duration: Duration) -> Self {
        Self {
            jti: Uuid::new_v4(),
            user_id,
            login: login.to_string(),
            exp: jsonwebtoken::get_current_timestamp() + duration.whole_seconds().abs() as u64,
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for Claims {
    type Rejection = AppError;

    async fn from_request_parts(
        req: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        verify_token::<Self>(req, state).await
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RefreshClaims {
    pub jti: Uuid,
    pub user_id: Uuid,
    pub login: String,
    pub exp: u64,
}

impl RefreshClaims {
    pub fn new(user_id: Uuid, login: &str, duration: Duration) -> Self {
        Self {
            jti: Uuid::new_v4(),
            user_id,
            login: login.to_string(),
            exp: jsonwebtoken::get_current_timestamp() + duration.whole_seconds().abs() as u64,
        }
    }
}

#[async_trait]
impl FromRequestParts<AppState> for RefreshClaims {
    type Rejection = AppError;

    async fn from_request_parts(
        req: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        verify_token::<Self>(req, state).await
    }
}

async fn verify_token<T>(req: &mut Parts, state: &AppState) -> Result<T, AppError>
where
    T: AuthToken,
{
    let jwt_key = T::get_jwt_key(&state.token_ext).await;

    // get extensions - PgPool
    let pool = state.postgres.clone();

    // get extensions - CookieJar
    let jar = CookieJar::from_request_parts(req, state)
        .await
        .context("Failed to fetch cookie jar")?;

    let cookie = T::get_jwt_cookie(jar).await?;

    let claims = T::decode_jwt(cookie.value(), jwt_key).await?;

    let res = claims.check_if_in_blacklist(&pool).await?;

    match res {
        true => Err(AppError::exp(StatusCode::UNAUTHORIZED, "Invalid token")),
        false => Ok(claims),
    }
}

#[derive(Serialize, Deserialize, Validate)]
pub struct LoginCredentials {
    #[validate(email)]
    pub email: String,
    pub password: String,
}

impl LoginCredentials {
    pub fn new(email: &str, password: &str) -> Self {
        Self {
            email: email.into(),
            password: password.into(),
        }
    }
}

#[derive(Serialize, Deserialize, Validate)]
pub struct RegisterCredentials {
    #[validate(email)]
    pub email: String,
    pub password: String,
    pub username: String,
}

impl RegisterCredentials {
    pub fn new(email: &str, password: &str, username: &str) -> Self {
        Self {
            email: email.into(),
            password: password.into(),
            username: username.into(),
        }
    }
}
