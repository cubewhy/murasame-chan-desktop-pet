use std::net::TcpListener;

use tts::{config::AppConfig, startup::create_server, telemetry::{get_subscriber, init_subscriber}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init dotenv
    dotenvy::dotenv()?;
    // Init logger
    let subscriber = get_subscriber("tts-backend", "info", std::io::stdout);
    init_subscriber(subscriber);

    // Load configuration
    let config = AppConfig::from_env()?;

    // create the server
    let listener = TcpListener::bind(&config.servlet.address)?;
    let server = create_server(listener, config)?;

    // run the server
    server.await?;

    Ok(())
}
