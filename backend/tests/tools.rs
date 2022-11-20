use backend::{app, database::get_database_pool};
use backend::configuration::get_config;
use dotenv::dotenv;
use reqwest::Client;
use std::net::{SocketAddr, TcpListener};
pub async fn spawn_app() -> SocketAddr {
    dotenv().ok();

    let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0))).unwrap();
    let addr = listener.local_addr().unwrap();

    let config = get_config().expect("Failed to read config");

    tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .unwrap()
            .serve(app(get_database_pool(&config.test_database.connection_string()).await)
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
