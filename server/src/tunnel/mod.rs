use anyhow::Result;
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

pub async fn handle_conn(mut conn: TcpStream, db_pool: DbPool) -> Result<()> {
    match read_hello(&mut conn).await {
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
                return Ok(());
            }
            conn.write_all(&bincode::serialize(&ServerPacket::Hello(Hello {
                protocol_version: CURRENT_PROTO_VERSION,
            }))?)
            .await?;
            conn.write_all(b"\n").await?;
            conn.flush().await?;
        }
        Err(e) => return handle_read_error(&mut conn, e).await,
    };

    use crate::schema::secret_keys::dsl::*;

    match read_auth(&mut conn).await {
        Ok(key) => {
            let mut db_conn = db_pool.get()?;

            let mut hasher = Sha256::new();
            hasher.update(key);
            let key_hash = format!("{:x}", hasher.finalize());

            let key_found: Vec<SecretKey> = secret_keys
                .select(SecretKey::as_select())
                .filter(secret_key_hash.eq(key_hash))
                .load(&mut db_conn)?;

            if key_found.is_empty() {
                conn.write_all(&bincode::serialize(&ServerPacket::Unauthorized)?)
                    .await?;
                conn.write_all(b"\n").await?;
                conn.flush().await?;
                return Ok(());
            }

            conn.write_all(&bincode::serialize(&ServerPacket::AuthSuccess)?)
                .await?;
            conn.write_all(b"\n").await?;
            conn.flush().await?;

            tracing::info!("auth successful");
        }
        Err(e) => return handle_read_error(&mut conn, e).await,
    }

    Ok(())
}

async fn handle_read_error(conn: &mut TcpStream, e: ReadError) -> Result<()> {
    match e {
        ReadError::Io(e) => return Err(e.into()),
        ReadError::Deserialization(_) | ReadError::InvalidPacket => {
            conn.write_all(&bincode::serialize(&ServerPacket::from_error(
                WombatError::InvalidPacket,
            ))?)
            .await?;
            conn.write_all(b"\n").await?;
            conn.flush().await?;
            return Ok(());
        }
    }
}
