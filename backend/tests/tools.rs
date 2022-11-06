use backend::app;
use reqwest::Client;
use sqlx::{PgPool, PgConnection, Connection, Executor};
use uuid::Uuid;
use std::net::{TcpListener, SocketAddr};

pub async fn spawn_app() -> SocketAddr {
    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
    let addr = listener.local_addr().unwrap();

    let db_data = DatabaseConfig {
        username: "postgres".to_string(),
        address: "localhost".to_string(),
        port: "5432".to_string(),
        name: Uuid::new_v4().to_string(),
    };

    let pool = config_db(db_data).await;

    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app(pool).await.into_make_service())
            .await
            .unwrap()
    });

    addr
}

pub fn client() -> Client {
    Client::builder().cookie_store(true).build().expect("Failed to build reqwest client")
}

pub async fn config_db(config: DatabaseConfig) -> PgPool {
    let mut connection = PgConnection::connect(&config.to_db_url_no_name())
        .await
        .expect("Failed to connect to Postgres");
    connection
        .execute(format!(r#"create database "{}";"#, config.name).as_str())
        .await
        .expect("Failed to create database.");

    let url = &config.to_db_url();
    let pool = PgPool::connect(&url).await.unwrap();
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to migrate the database");

    pool
}

pub struct DatabaseConfig {
    pub username: String,
    pub address: String,
    pub port: String,
    pub name: String,
}

impl DatabaseConfig {
    fn to_db_url(&self) -> String {
        format!("postgresql://{}@{}:{}/{}", self.username, self.address, self.port, self.name)
    }

    fn to_db_url_no_name(&self) -> String {
        format!("postgresql://{}@{}:{}", self.username, self.address, self.port)
    }
}
