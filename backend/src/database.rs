use sqlx::PgPool;

pub async fn get_database_pool(url: &str) -> PgPool {
    let pool = PgPool::connect(url)
        .await
        .expect("Cannot establish database connection");
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Migration error");

    pool
}
