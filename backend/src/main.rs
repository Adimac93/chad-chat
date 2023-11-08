pub mod configuration;
pub mod errors;
pub mod modules;
pub mod routes;
pub mod state;
pub mod utils;

use crate::modules::extractors::addr::ClientAddr;
use crate::{configuration::get_config, routes::app};
use dotenv::dotenv;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use tracing_subscriber::{EnvFilter, Layer};

#[macro_use]
pub extern crate tracing;

#[tokio::main]
async fn main() {
    dotenv().ok();
    let config = get_config().expect("Failed to read configuration");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .without_time()
                .pretty()
                .with_line_number(true)
                .with_filter(EnvFilter::builder().parse("backend=trace").unwrap()),
        )
        .with(console_subscriber::spawn())
        .init();

    let addr = config.app.get_addr();

    info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(
            app(config, None)
                .await
                .into_make_service_with_connect_info::<ClientAddr>(),
        )
        .await
        .expect("Failed to run axum server");
}
