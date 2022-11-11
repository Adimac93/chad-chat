use sqlx::PgPool;

pub async fn get_database_pool(url: &str) -> PgPool {
    PgPool::connect(url)
        .await
        .expect("Cannot establish database connection")
}
