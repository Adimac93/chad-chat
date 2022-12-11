use secrecy::ExposeSecret;
use sqlx::{migrate, PgPool};

use crate::configuration::DatabaseSettings;

pub async fn get_database_pool(config: DatabaseSettings) -> PgPool {
    let pool = PgPool::connect(&config.get_connection_string())
        .await
        .expect("Cannot establish database connection");
    if config.is_migrating() {
        migrate!("./migrations")
            .run(&pool)
            .await
            .expect("Auto migration failed");
    }
    pool
}
