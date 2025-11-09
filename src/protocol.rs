use crate::codec::{VarInt, VarIntString};
use byteorder::{NetworkEndian, ReadBytesExt};
use std::io;
use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;

#[derive(Debug)]
pub struct Packet {
    pub length: i32,
    pub id: i32,
    pub message: Message,
}

impl Packet {
    pub async fn read_from<R: AsyncRead + Unpin>(
        reader: &mut R,
        connection_path: Option<HandshakeIntent>,
    ) -> io::Result<Self> {
        let length = i32::from_var_int(reader).await?;
        let id = i32::from_var_int(reader).await?;

        let message = match connection_path {
            None => match id {
                0x00 => Message::Handshake(Handshake::read_from(reader).await?),
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("Unknown packet ID: {:x}", id),
                    ));
                }
            },
            Some(HandshakeIntent::Status) => Message::StatusRequest(StatusRequest {}),
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Unsupported connection path",
                ));
            }
        };

        Ok(Packet {
            length,
            id,
            message,
        })
    }
}

#[derive(Debug)]
pub enum MessageType {
    Handshake,
    StatusRequest,
    StatusResponse,
}

#[derive(Debug)]
pub enum Message {
    Handshake(Handshake),
    StatusRequest(StatusRequest),
}

#[derive(Debug)]
pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub intent: HandshakeIntent,
}

impl Handshake {
    pub async fn read_from<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Self> {
        let protocol_version = i32::from_var_int(reader).await?;
        let server_address = String::from_var_int_string(reader).await?;
        let server_port = reader.read_u16().await?;
        let intent = HandshakeIntent::read_from(reader).await?;

        Ok(Handshake {
            protocol_version,
            server_address,
            server_port,
            intent,
        })
    }
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub enum HandshakeIntent {
    Status,
    Login,
    Transfer,
}

impl HandshakeIntent {
    pub async fn read_from<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Self> {
        let intent = i32::from_var_int(reader).await?;
        match intent {
            1 => Ok(HandshakeIntent::Status),
            2 => Ok(HandshakeIntent::Login),
            3 => Ok(HandshakeIntent::Transfer),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Unknown handshake intent",
            )),
        }
    }
}

#[derive(Debug)]
pub struct StatusRequest {}


// #[derive(Debug)]
// pub struct StatusResponse<'a> {
//
// }
