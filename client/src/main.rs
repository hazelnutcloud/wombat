use std::time::Duration;

use anyhow::{bail, Context};
use hyper::body::Incoming;
use hyper::server::conn::http2;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::{TokioExecutor, TokioIo, TokioTimer};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use tracing_subscriber::filter::LevelFilter;
use wombat_client::config::{get_config, write_config};
use wombat_server::protocol::{ClientPacket, Hello, ServerPacket, CURRENT_PROTO_VERSION};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();
    let (config, should_write_config) = get_config()?;

    if should_write_config {
        write_config(&config)?;
    };

    let mut conn =
        TcpStream::connect(format!("{}:{}", config.server_hostname, config.tunnel_port)).await?;

    handshake_server(&mut conn, &config.secret_key).await?;

    let io = TokioIo::new(conn);
    let exec = TokioExecutor::new();

    let http_client = Client::builder(TokioExecutor::new()).build_http();

    http2::Builder::new(exec)
        .keep_alive_interval(Some(Duration::from_secs(60)))
        .timer(TokioTimer::new())
        .serve_connection(
            io,
            service_fn(|req| relay_requests(req, http_client.clone())),
        )
        .await?;

    Ok(())
}

async fn relay_requests(
    request: Request<Incoming>,
    http_client: Client<HttpConnector, Incoming>,
) -> Result<Response<Incoming>, anyhow::Error> {
    let (parts, body) = request.into_parts();
    println!("{parts:?}");
    let mut request = Request::builder()
        .method(parts.method)
        .uri(parts.uri)
        .body(body)?;
    *request.headers_mut() = parts.headers;
    http_client.request(request).await.map_err(|e| e.into())
}

async fn handshake_server(conn: &mut TcpStream, secret_key: &str) -> anyhow::Result<()> {
    conn.write_all(&bincode::serialize(&ClientPacket::Hello(Hello {
        protocol_version: CURRENT_PROTO_VERSION,
    }))?)
    .await?;
    conn.flush().await?;

    read_hello(conn).await?;

    let secret_key: [u8; 32] = secret_key.as_bytes().try_into().context("Invalid key")?;

    conn.write_all(&bincode::serialize(&ClientPacket::Auth(secret_key))?)
        .await?;
    conn.flush().await?;

    read_auth(conn).await?;

    println!("Handshake complete!");

    Ok(())
}

async fn read_server_packet(conn: &mut TcpStream) -> anyhow::Result<ServerPacket> {
    let mut reader = BufReader::new(conn);
    let mut buf = String::new();

    reader.read_line(&mut buf).await?;

    Ok(bincode::deserialize(buf.as_bytes())?)
}

async fn read_hello(conn: &mut TcpStream) -> anyhow::Result<()> {
    let response = read_server_packet(conn).await?;

    match response {
        ServerPacket::Hello(_) => return Ok(()),
        ServerPacket::Error(e) => match e {
            wombat_server::protocol::WombatError::ProtocolVersionMismatch { server_version: _ } => {
                bail!("Outdated wombat client. Please update to connect to this server!");
            }
            wombat_server::protocol::WombatError::InvalidPacket => bail!("Invalid packet"),
        },
        packet => bail!("Unexpected packet: {packet:?}"),
    }
}

async fn read_auth(conn: &mut TcpStream) -> anyhow::Result<()> {
    let response = read_server_packet(conn).await?;

    match response {
        ServerPacket::AuthSuccess => return Ok(()),
        ServerPacket::Unauthorized => bail!("Unauthorized!"),
        packet => bail!("Unexpected packet: {packet:?}"),
    }
}
