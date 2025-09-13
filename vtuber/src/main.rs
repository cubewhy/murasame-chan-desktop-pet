use vtuber::run;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv()?;
    run().await?;

    Ok(())
}
