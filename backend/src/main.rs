use backend::{app, configuration::get_config, database::get_database_pool};
use dotenv::dotenv;
use std::net::SocketAddr;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenv().ok();

    let config = get_config().expect("Failed to read configuration");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let addr = SocketAddr::from((config.addr.ip, config.addr.port));
    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(
            app(get_database_pool(&config.database.connection_string()).await)
                .await
                .into_make_service(),
        )
        .await
        .expect("Failed to run axum server");
}
