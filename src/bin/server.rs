use anyhow::Result;
use wombat::auth_server::run_auth_server;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt::init();

    let _ = tokio::spawn(run_auth_server()).await?;

    Ok(())
}
