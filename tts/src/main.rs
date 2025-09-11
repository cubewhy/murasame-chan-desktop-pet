use std::net::TcpListener;

use tts::startup::create_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // create the server
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    let server = create_server(listener)?;

    // run the server
    server.await?;

    Ok(())
}
