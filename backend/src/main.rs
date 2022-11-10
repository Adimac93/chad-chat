use backend::{app, database::get_database_pool};
use dotenv::dotenv;
use std::net::SocketAddr;
use tracing::warn;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    std::env::var("DATABASE_URL").expect("No DATABASE_URL env var found");
    if std::env::var("TOKEN_SECRET").is_err() {
        warn!("No TOKEN_SECRET env var found");
    }

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app(get_database_pool().await).await.into_make_service())
        .await
        .expect("Failed to run axum server");
}
