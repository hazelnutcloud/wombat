mod app;
mod redirect;
mod signup;

use std::io;

use tokio::net::TcpListener;

pub async fn run_auth_server() -> io::Result<()> {
    let app = app::get_app();
    let host = std::env::var("HOST").unwrap_or("0.0.0.0".into());
    let port = std::env::var("PORT").unwrap_or("8080".into());

    let listener = TcpListener::bind(format!("{host}:{port}")).await?;
    println!("🚀 Auth server running on http://{host}:{port}");

    axum::serve(listener, app).await
}
