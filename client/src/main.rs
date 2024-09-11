use anyhow::{bail, Context, Result};
use clap::Parser;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};
use wombat_client::config::{get_config, write_config, Cli};
use wombat_server::protocol::{ClientPacket, Hello, ServerPacket, CURRENT_PROTO_VERSION};

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let (config, should_write_config) = get_config(cli)?;

    if should_write_config {
        write_config(&config)?;
    };

    let mut conn =
        TcpStream::connect(format!("{}:{}", config.server_hostname, config.tunnel_port)).await?;

    handshake(&mut conn, &config.secret_key).await?;

    Ok(())
}

async fn handshake(conn: &mut TcpStream, secret_key: &str) -> Result<()> {
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

async fn read_server_packet(conn: &mut TcpStream) -> Result<ServerPacket> {
    let mut reader = BufReader::new(conn);
    let mut buf = String::new();

    reader.read_line(&mut buf).await?;

    Ok(bincode::deserialize(buf.as_bytes())?)
}

async fn read_hello(conn: &mut TcpStream) -> Result<()> {
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

async fn read_auth(conn: &mut TcpStream) -> Result<()> {
    let response = read_server_packet(conn).await?;

    match response {
        ServerPacket::AuthSuccess => return Ok(()),
        ServerPacket::Unauthorized => bail!("Unauthorized!"),
        packet => bail!("Unexpected packet: {packet:?}"),
    }
}
