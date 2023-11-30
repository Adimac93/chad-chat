use backend::{configuration::get_config, routes::app};
use dotenv::dotenv;
use redis::{cmd, Client as RedisClient, aio::ConnectionManager, Value};
use reqwest::Client;
use sqlx::PgPool;
use std::net::{SocketAddr, TcpListener};

#[cfg(test)]
const DEFAULT_BASE_REDIS_URL: &str = "redis://127.0.0.1:6379/";

async fn spawn_app(db: PgPool) -> SocketAddr {
    dotenv().ok();

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
    let addr = listener.local_addr().unwrap();

    let settings = get_config().unwrap();
    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app(settings, Some(db)).await.into_make_service())
            .await
            .unwrap()
    });

    addr
}

pub struct AppData {
    pub addr: SocketAddr,
}

impl AppData {
    pub async fn new(db: PgPool) -> Self {
        Self {
            addr: spawn_app(db).await,
        }
    }

    pub fn client(&self) -> Client {
        Client::builder()
            .cookie_store(true)
            .build()
            .expect("Failed to build reqwest client")
    }
}

#[cfg(test)]
pub async fn add_redis<T>(db_number: i32, fixtures: T) -> ConnectionManager
where
    T: IntoIterator,
    <T as IntoIterator>::Item: Into<String> {
    dotenv::dotenv().ok();

    let base_redis_url = std::env::var("BASE_REDIS_URL").unwrap_or(DEFAULT_BASE_REDIS_URL.to_string());

    let client =
        RedisClient::open(format!("{base_redis_url}{db_number}")).expect("Cannot establish redis connection");
    
    let mut rd = client
        .get_tokio_connection_manager()
        .await
        .expect("Failed to get redis connection manager");

    rd.send_packed_command(&cmd("FLUSHDB")).await.unwrap();

    for x in fixtures {
        let s: String = x.into();
        if s.trim().is_empty() {
            continue;
        };
        let args = parse_args(s);
        let _: Value = cmd(&args[0]).arg(&args[1..]).query_async(&mut rd).await.unwrap();
    };

    rd
}

#[cfg(test)]
fn parse_args(s: String) -> Vec<String> {
    s.split_whitespace().filter(|&x| x != "").map(|x| x.to_string()).collect()
}
