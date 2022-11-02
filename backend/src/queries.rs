use axum::{extract, Extension, Json, http::StatusCode, response::{Response, Html}};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Serialize, Deserialize};
use sqlx::{pool::PoolConnection, query, query_as, PgPool, Pool, Postgres};
use tracing::info;
use uuid::Uuid;
use argon2::{hash_encoded, verify_encoded};
use thiserror::Error;
use anyhow::{self, Context};
use zxcvbn;

#[derive(Error, Debug)]
pub enum AuthError {
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
    if !is_strong(password, &[&login]) {
        return Err(AuthError::WeakPassword)
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


pub async fn post_login_user(
    pool: Extension<PgPool>,
    user: extract::Json<AuthUser>,
) -> Result<(), StatusCode> {
    let mut conn = pool.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    login_user(&mut conn, &user.login, &user.password).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(())
}

pub async fn login_user(
    conn: &mut PoolConnection<Postgres>,
    login: &str,
    password: &str,
) -> Result<Uuid, AuthError> {
    let res = query!("
    select * from users where login = $1
    ",
    login)
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