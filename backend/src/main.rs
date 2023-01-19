use std::net::SocketAddr;

use backend::{app, configuration::get_config, database::get_postgres_pool, utils::roles::models::{QueryPrivileges, Privileges}};
use dotenv::dotenv;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    println!("{:?}", serde_json::to_string(&QueryPrivileges::from(Privileges::max())));
    dotenv().ok();

    let config = get_config().expect("Failed to read configuration");

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "backend=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let addr = config.app.get_addr();

    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(
            app(config, None)
                .await
                .into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("Failed to run axum server");
}
