use axum::{extract, Extension, Json, http::StatusCode, response::{Response, Html}};
use serde::{Serialize, Deserialize};
use sqlx::{pool::PoolConnection, query, query_as, PgPool, Pool, Postgres};
use tracing::info;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct AuthUser {
    pub login: String,
    pub password: String,
}

pub async fn get_databse_pool() -> PgPool {
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
    login: &String,
    password: &String,
) -> Result<(),sqlx::Error> {
    let res = query!(
        r#"
            insert into users (login, password)
            values ($1, $2)
        "#,
        login,
        password
    )
    .execute(conn)
    .await?;

    info!("{res:?}");
    Ok(())
}
