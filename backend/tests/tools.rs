﻿use backend::app;
use dotenv::dotenv;
use reqwest::Client;
use sqlx::PgPool;
use std::net::{SocketAddr, TcpListener};
pub async fn spawn_app(db: PgPool) -> SocketAddr {
    dotenv().ok();

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app(db)
                .await
                .into_make_service())
            .await
            .unwrap()
    });

    addr
}

pub fn client() -> Client {
    Client::builder()
        .cookie_store(true)
        .build()
        .expect("Failed to build reqwest client")
}

pub struct AppData {
    pub addr: SocketAddr,
    pub client: Client,
}

impl AppData {
    pub async fn new(db: PgPool) -> Self {
        Self {
            addr: spawn_app(db).await,
            client: client(),
        }
    }
}
