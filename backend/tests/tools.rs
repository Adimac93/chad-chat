use backend::app;
use dotenv::dotenv;
use reqwest::Client;
use sqlx::PgPool;
use std::net::{SocketAddr, TcpListener};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
pub async fn spawn_app(db: PgPool) -> SocketAddr {
    dotenv().ok();

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
    let addr = listener.local_addr().unwrap();

    // !panics when set for the second time - can't run multiple tests setting tracing subscriber

    // tracing_subscriber::registry()
    //     .with(tracing_subscriber::EnvFilter::new(
    //         std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug".into()),
    //     ))
    //     .with(tracing_subscriber::fmt::layer())
    //     .init();

    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app(db).await.into_make_service())
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
