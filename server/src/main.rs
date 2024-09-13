use anyhow::{Context, Result};
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use poise::{FrameworkOptions, PrefixFrameworkOptions};
use serenity::all::GatewayIntents;
use std::env;
use tokio::{
    net::{TcpListener, TcpStream},
    sync::mpsc::Sender,
};
use tracing_subscriber::EnvFilter;
use wombat_server::{
    auth::{self, AppVariables},
    connection::{DiscordFetchRequest, TunnelManager},
    discord::{fetch, Data},
    tunnel::handshake_client,
    utils::DbPool,
};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let auth_host = env::var("AUTH_HOST").unwrap_or("0.0.0.0".into());
    let auth_port = env::var("AUTH_PORT").unwrap_or("8080".into());
    let tunneler_host = env::var("TUNNELER_HOST").unwrap_or("0.0.0.0".into());
    let tunneler_port = env::var("TUNNELER_PORT").unwrap_or("9090".into());
    let client_id = env::var("DISCORD_CLIENT_ID").expect("DISCORD_CLIENT_ID not set");
    let client_secret = env::var("DISCORD_CLIENT_SECRET").expect("DISCORD_CLIENT_SECRET not set");
    let redirect_uri =
        env::var("REDIRECT_URI").unwrap_or("http://localhost:8080/auth/redirect".into());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");
    let bot_token = env::var("DISCORD_BOT_TOKEN").expect("DISCORD_BOT_TOKEN not set");

    let db_pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(database_url))?;

    run_migrations(&db_pool);

    let (tunnel_manager, conn_tx, req_tx) = TunnelManager::new();

    tokio::select! {
      auth_server = run_auth_server(auth_host, auth_port, db_pool.clone(), AppVariables {
          client_id,
          client_secret,
          redirect_uri,
      }) => auth_server,
      tunneler = run_tunneler(tunneler_host, tunneler_port, db_pool.clone(), conn_tx) => tunneler,
      discord_bot = run_discord_bot(bot_token, req_tx, db_pool) => discord_bot,
      tunnel_manager = tunnel_manager.run() => tunnel_manager
    }
}

async fn run_auth_server(
    host: String,
    port: String,
    db_pool: DbPool,
    app_variables: AppVariables,
) -> Result<()> {
    let app = auth::app(db_pool, app_variables);

    let listener = TcpListener::bind(format!("{host}:{port}")).await?;
    tracing::info!("🚀 Auth server running on http://{host}:{port}");

    axum::serve(listener, app)
        .await
        .context("failed to start auth server")
}

fn run_migrations(pool: &DbPool) {
    let mut conn = pool.get().unwrap();
    conn.run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");
}

async fn run_tunneler(
    host: String,
    port: String,
    db_pool: DbPool,
    conn_tx: Sender<(String, TcpStream)>,
) -> Result<()> {
    let listener = TcpListener::bind(format!("{host}:{port}")).await?;

    loop {
        let (mut conn, _) = listener.accept().await?;

        match handshake_client(&mut conn, db_pool.clone()).await {
            Ok(user_id) => {
                if let Err(e) = conn_tx.send((user_id, conn)).await {
                    tracing::error!("error transmitting new conn: {e}")
                }
            }
            Err(e) => {
                tracing::error!("error handling client conn: {e}");
            }
        }
    }
}

async fn run_discord_bot(
    bot_token: String,
    req_tx: Sender<DiscordFetchRequest>,
    db_pool: DbPool,
) -> Result<()> {
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
        .setup(|ctx, _, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { req_tx, db_pool })
            })
        })
        .build();

    let mut client = serenity::Client::builder(&bot_token, intents)
        .framework(framework)
        .await?;

    client.start().await.map_err(|e| e.into())
}
