use anyhow::Context;
use diesel::{
    r2d2::{ConnectionManager, Pool},
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::env;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;
use wombat::{
    auth::{self, AppVariables},
    utils::DbPool,
};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let host = env::var("HOST").unwrap_or("0.0.0.0".into());
    let port = env::var("PORT").unwrap_or("8080".into());
    let client_id = env::var("DISCORD_CLIENT_ID").expect("DISCORD_CLIENT_ID not set");
    let client_secret = env::var("DISCORD_CLIENT_SECRET").expect("DISCORD_CLIENT_SECRET not set");
    let redirect_uri =
        env::var("REDIRECT_URI").unwrap_or("http://localhost:8080/auth/redirect".into());
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not set");

    let db_pool = Pool::builder()
        .max_size(1)
        .build(ConnectionManager::<SqliteConnection>::new(database_url))?;

    run_migrations(&db_pool);

    tokio::select! {
      auth_server = run_auth_server(host, port, db_pool.clone(), AppVariables {
        client_id,
        client_secret,
        redirect_uri,
    }) => {
        auth_server.context("failed to run auth server")
      }
    }
}

async fn run_auth_server(
    host: String,
    port: String,
    db_pool: DbPool,
    app_variables: AppVariables,
) -> std::io::Result<()> {
    let app = auth::app(db_pool, app_variables);

    let listener = TcpListener::bind(format!("{host}:{port}")).await?;
    tracing::info!("🚀 Auth server running on http://{host}:{port}/auth/signup");

    axum::serve(listener, app).await
}

fn run_migrations(pool: &DbPool) {
    let mut conn = pool.get().unwrap();
    conn.run_pending_migrations(MIGRATIONS)
        .expect("failed to run migrations");
}
