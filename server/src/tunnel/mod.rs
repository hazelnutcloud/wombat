use anyhow::{bail, Result};
use diesel::prelude::*;
use sha2::{Digest, Sha256};
use tokio::{io::AsyncWriteExt, net::TcpStream};

use crate::{
    models::SecretKey,
    protocol::{
        read_auth, read_hello, Hello, ReadError, ServerPacket, WombatError, CURRENT_PROTO_VERSION,
    },
    utils::DbPool,
};

pub async fn handshake_client(conn: &mut TcpStream, db_pool: DbPool) -> Result<String> {
    match read_hello(conn).await {
        Ok(hello) => {
            if !hello.is_valid() {
                conn.write_all(&bincode::serialize(&ServerPacket::from_error(
                    WombatError::ProtocolVersionMismatch {
                        server_version: CURRENT_PROTO_VERSION,
                    },
                ))?)
                .await?;
                conn.write_all(b"\n").await?;
                conn.flush().await?;
                bail!("Invalid client protocol version");
            }
            conn.write_all(&bincode::serialize(&ServerPacket::Hello(Hello {
                protocol_version: CURRENT_PROTO_VERSION,
            }))?)
            .await?;
            conn.write_all(b"\n").await?;
            conn.flush().await?;
        }
        Err(e) => return Err(handle_read_error(conn, e).await.unwrap_err()),
    };

    use crate::schema::secret_keys::dsl::*;

    let key_hash = match read_auth(conn).await {
        Ok(key) => {
            let mut db_conn = db_pool.get()?;

            let mut hasher = Sha256::new();
            hasher.update(key);
            let key_hash = format!("{:x}", hasher.finalize());

            let key_found: Vec<SecretKey> = secret_keys
                .select(SecretKey::as_select())
                .filter(secret_key_hash.eq(&key_hash))
                .load(&mut db_conn)?;

            if key_found.is_empty() {
                conn.write_all(&bincode::serialize(&ServerPacket::Unauthorized)?)
                    .await?;
                conn.write_all(b"\n").await?;
                conn.flush().await?;
                bail!("Client unauthorized");
            }

            conn.write_all(&bincode::serialize(&ServerPacket::AuthSuccess)?)
                .await?;
            conn.write_all(b"\n").await?;
            conn.flush().await?;

            tracing::info!("auth successful");
            key_hash
        }
        Err(e) => return Err(handle_read_error(conn, e).await.unwrap_err()),
    };

    Ok(key_hash)
}

async fn handle_read_error(conn: &mut TcpStream, e: ReadError) -> Result<()> {
    match e {
        ReadError::Io(e) => Err(e.into()),
        ReadError::Deserialization(_) | ReadError::InvalidPacket => {
            conn.write_all(&bincode::serialize(&ServerPacket::from_error(
                WombatError::InvalidPacket,
            ))?)
            .await?;
            conn.write_all(b"\n").await?;
            conn.flush().await?;
            bail!("Invalid client packet");
        }
    }
}
