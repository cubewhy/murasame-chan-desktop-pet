use env_logger::Env;
use vtuber::run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    run().await?;

    Ok(())
}
