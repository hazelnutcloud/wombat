use anyhow::Result;
use clap::Parser;
use wombat_client::config::{get_config, write_config, Cli};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let (config, should_write_config) = get_config(cli)?;

    if should_write_config {
        write_config(&config)?;
    };

    println!("{config:?}");

    Ok(())
}
