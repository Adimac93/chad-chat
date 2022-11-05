use sqlx::PgPool;

pub async fn get_database_pool() -> PgPool {
    let url = &std::env::var("DATABASE_URL").expect("Cannot find database url");
    PgPool::connect(url).await.expect("Cannot establish database connection")
}