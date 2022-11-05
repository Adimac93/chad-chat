use secrecy::{ExposeSecret, SecretString};
use sqlx::PgPool;

pub async fn get_database_pool() -> PgPool {
    let url = SecretString::new(std::env::var("DATABASE_URL").expect("Cannot find database url"));
    PgPool::connect(url.expose_secret()).await.unwrap()
}