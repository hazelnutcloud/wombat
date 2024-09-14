use anyhow::Result;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use poise::{FrameworkOptions, PrefixFrameworkOptions};
use serenity::all::GatewayIntents;
use std::env;
use tokio::sync::mpsc::{self, Sender};
use tracing_subscriber::EnvFilter;
use wombat::discord::fetch::{fetch, Data, DiscordFetchRequest};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let bot_token = env::var("DISCORD_BOT_TOKEN").expect("DISCORD_BOT_TOKEN not set");

    let (req_tx, mut req_rx) = mpsc::channel(100);

    tokio::spawn(run_discord_bot(bot_token, req_tx));

    let http_client = Client::builder(TokioExecutor::new()).build_http();

    while let Some((req, res_tx)) = req_rx.recv().await {
        let http_client = http_client.clone();
        tokio::spawn(async move {
            let response = http_client.request(req).await.map_err(|e| e.to_string());
            res_tx.send(response).expect("Error sending response");
        });
    }

    Ok(())
}

async fn run_discord_bot(bot_token: String, req_tx: Sender<DiscordFetchRequest>) {
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT;

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            commands: vec![fetch()],
            prefix_options: PrefixFrameworkOptions {
                prefix: Some("~".into()),
                ..Default::default()
            },
            ..Default::default()
        })
        .setup(|_, _, _| Box::pin(async move { Ok(Data { req_tx }) }))
        .build();

    let mut client = serenity::Client::builder(&bot_token, intents)
        .framework(framework)
        .await
        .expect("Error while initializing discord bot");

    client
        .start()
        .await
        .expect("Error while starting discord bot");
}
