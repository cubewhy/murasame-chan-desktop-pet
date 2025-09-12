use std::net::TcpListener;

use tts::{startup::create_server, telemetry::{get_subscriber, init_subscriber}};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Init logger
    let subscriber = get_subscriber("zero2prod", "info", std::io::stdout);
    init_subscriber(subscriber);

    // create the server
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    let server = create_server(listener)?;

    // run the server
    server.await?;

    Ok(())
}
