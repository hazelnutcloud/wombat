use std::{io, sync::OnceLock};

use serde::{Deserialize, Serialize};
use tokio::{io::AsyncReadExt, net::TcpStream};

pub const CURRENT_PROTO_VERSION: u8 = 1;
pub const KEY_WIDTH_BYTES: usize = 32;

struct PacketLength {
    hello: usize,
    auth: usize,
}

impl PacketLength {
    fn get() -> &'static PacketLength {
        static PACKET_LENGTH: OnceLock<PacketLength> = OnceLock::new();
        PACKET_LENGTH.get_or_init(|| PacketLength {
            hello: bincode::serialized_size(&ClientPacket::Hello(Hello {
                protocol_version: CURRENT_PROTO_VERSION,
            }))
            .unwrap() as usize,
            auth: bincode::serialized_size(&ClientPacket::Auth([0u8; KEY_WIDTH_BYTES])).unwrap()
                as usize,
        })
    }
}

#[derive(Serialize, Deserialize)]
pub enum ServerPacket {
    Error(WombatError),
    Unauthorized
}

impl ServerPacket {
    pub fn from_error(error: WombatError) -> ServerPacket {
        ServerPacket::Error(error)
    }
}

#[derive(Serialize, Deserialize)]
pub enum WombatError {
    ProtocolVersionMismatch { server_version: u8 },
    InvalidPacket,
}

#[derive(Serialize, Deserialize)]
pub enum ClientPacket {
    Hello(Hello),
    Auth([u8; KEY_WIDTH_BYTES]),
}

#[derive(Serialize, Deserialize)]
pub struct Hello {
    protocol_version: u8,
}

impl Hello {
    pub fn is_valid(&self) -> bool {
        self.protocol_version == CURRENT_PROTO_VERSION
    }
}

pub enum ReadError {
    Io(io::Error),
    Deserialization(bincode::Error),
    InvalidPacket,
}

pub async fn read_client_packet(
    conn: &mut TcpStream,
    buf_len: usize,
) -> Result<ClientPacket, ReadError> {
    let mut buf: Vec<u8> = vec![0; buf_len];

    conn.read_exact(&mut buf)
        .await
        .map_err(|e| ReadError::Io(e))?;

    bincode::deserialize(&buf).map_err(|e| ReadError::Deserialization(e))
}

pub async fn read_hello(conn: &mut TcpStream) -> Result<Hello, ReadError> {
    match read_client_packet(conn, PacketLength::get().hello).await {
        Ok(ClientPacket::Hello(hello)) => Ok(hello),
        Ok(_) => Err(ReadError::InvalidPacket),
        Err(e) => Err(e),
    }
}

pub async fn read_auth(conn: &mut TcpStream) -> Result<[u8; KEY_WIDTH_BYTES], ReadError> {
    match read_client_packet(conn, PacketLength::get().auth).await {
        Ok(ClientPacket::Auth(key)) => Ok(key),
        Ok(_) => Err(ReadError::InvalidPacket),
        Err(e) => Err(e),
    }
}
