use axum::{extract, Extension, Json, http::StatusCode, response::{Response, Html}};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Serialize, Deserialize};
use serde_json::{Value, json};
use sqlx::{pool::PoolConnection, query, query_as, PgPool, Pool, Postgres};
use tracing::info;
use uuid::Uuid;
use argon2::{hash_encoded, verify_encoded};
use thiserror::Error;
use anyhow::{self, Context};
use zxcvbn;
use jsonwebtoken::{encode, Header, EncodingKey, DecodingKey};

#[derive(Error, Debug)]
pub enum AuthError {
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Missing credential")]
    MissingCredential,
    #[error("User not found")]
    UserNotFound,
    #[error("Password is too weak")]
    WeakPassword,
    #[error("Incorrect user or password")]
    WrongUserOrPassword,
    #[error(transparent)]
    Unexpected(#[from] anyhow::Error)
}

#[derive(Serialize, Deserialize)]
pub struct AuthUser {
    pub login: String,
    pub password: String,
}

pub fn get_token_secret() -> String {
    dotenv::dotenv().ok();
    std::env::var("TOKEN_SECRET").expect("Cannot find token secret")
}

pub async fn get_database_pool() -> PgPool {
    dotenv::dotenv().ok();
    let url = &std::env::var("DATABASE_URL").expect("Cannot find database url");
    PgPool::connect(url).await.unwrap()
}

pub async fn post_register_user(
    pool: Extension<PgPool>,
    user: extract::Json<AuthUser>,
) -> Result<(),StatusCode>{
    let mut conn = pool.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    try_register_user(&mut conn, &user.login, &user.password).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

pub async fn try_register_user(
    conn: &mut PoolConnection<Postgres>,
    login: &str,
    password: &str,
) -> Result<(), AuthError> {
    if login.trim().is_empty() || password.trim().is_empty() {
        return Err(AuthError::MissingCredential);
    }

    if !is_strong(password, &[&login]) {
        return Err(AuthError::WeakPassword)
    }

    let user = query!("
        select * from users where login = $1
    ", login)
    .fetch_optional(&mut *conn)
    .await
    .context("Query failed")?;

    if let Some(_) = user {
        return Err(AuthError::UserAlreadyExists);
    }

    let hashed_pass = hash_pass(&password).context("Failed to hash pass")?;

    let res = query!(
        r#"
            insert into users (login, password)
            values ($1, $2)
        "#,
        login,
        hashed_pass
    )
    .execute(conn)
    .await
    .context("Query failed");

    info!("{res:?}");
    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub id: Uuid,
    pub exp: u64,
}

pub struct Keys {
    pub encoding: EncodingKey,
    pub decoding: DecodingKey,
}

impl Keys {
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
        }
    }
}

pub async fn post_login_user(
    pool: Extension<PgPool>,
    user: extract::Json<AuthUser>,
) -> Result<Json<Value>, StatusCode> {
    const ONE_HOUR_IN_SECONDS: u64 = 3600;

    let mut conn = pool.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let user_id = login_user(&mut conn, &user.login, &user.password).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let claims = Claims {
        id: user_id,
        exp: jsonwebtoken::get_current_timestamp() + ONE_HOUR_IN_SECONDS,
    };

    let token = encode(&Header::default(), &claims, &Keys::new(get_token_secret().as_bytes()).encoding)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(json!({ "access_token": token, "type": "Bearer" })))
}

pub async fn login_user(
    conn: &mut PoolConnection<Postgres>,
    login: &str,
    password: &str,
) -> Result<Uuid, AuthError> {
    if login.trim().is_empty() || password.trim().is_empty() {
        return Err(AuthError::MissingCredential);
    }

    let res = query!("
        select * from users where login = $1
    ", login)
    .fetch_optional(conn)
    .await
    .context("User query failed")?
    .ok_or(AuthError::UserNotFound)?;

    info!("{res:?}");
    if verify_encoded(&res.password, password.as_bytes()).context("Failed to verify password")? {
        Ok(res.id)
    } else {
        Err(AuthError::WrongUserOrPassword)
    }
}

fn hash_pass(pass: &str) -> Result<String, AuthError> {
    Ok(hash_encoded(pass.as_bytes(), random_salt().as_bytes(), &argon2::Config::default()).context("Failed to hash pass")?)
}

fn random_salt() -> String {
    let mut rng = thread_rng();
    (0..8).map(|_| rng.sample(Alphanumeric) as char).collect()
}

fn is_strong(user_password: &str, user_inputs: &[&str]) -> bool {
    let score = zxcvbn::zxcvbn(user_password, user_inputs);
    match score {
        Ok(s) => s.score() >= 3,
        Err(_) => false,
    }
}