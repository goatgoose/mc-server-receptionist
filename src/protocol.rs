use crate::codec::{VarInt, VarIntString};
use std::io;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::io::AsyncReadExt;
use serde_json::json;

#[derive(Debug)]
pub struct Packet<'a> {
    pub message: Message<'a>,
}

impl<'a> Packet<'a> {
    pub fn new(message: Message<'a>) -> Self {
        Packet {
            message,
        }
    }

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
            Some(HandshakeIntent::Status) => {
                match id {
                    0x00 => Message::StatusRequest(StatusRequest {}),
                    0x01 => Message::PingRequest(PingRequest::read_from(reader).await?),
                    _ => return Err(io::Error::new(io::ErrorKind::Unsupported, "Unrecognized status packet received"))
                }

            },
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Unsupported connection path",
                ));
            }
        };

        Ok(Packet {
            message,
        })
    }

    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        let mut buf = Vec::new();

        match &self.message {
            Message::StatusResponse(response) => {
                let packet_id = 0x00;
                packet_id.to_var_int(&mut buf).await?;
                response.write_to(&mut buf).await?;
            },
            Message::PingResponse(response) => {
                let packet_id = 0x01;
                packet_id.to_var_int(&mut buf).await?;
                response.write_to(&mut buf).await?;
            },
            _ => return Err(io::Error::new(io::ErrorKind::Unsupported, "Unimplemented message write")),
        }

        (buf.len() as i32).to_var_int(writer).await?;
        writer.write(buf.as_slice()).await?;

        writer.flush().await?;

        Ok(())
    }
}

#[derive(Debug)]
pub enum MessageType {
    Handshake,
    StatusRequest,
    StatusResponse,
}

#[derive(Debug)]
pub enum Message<'a> {
    Handshake(Handshake),
    StatusRequest(StatusRequest),
    StatusResponse(StatusResponse<'a>),
    PingRequest(PingRequest),
    PingResponse(PingResponse),
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

#[derive(Debug)]
pub struct StatusResponse<'a> {
    pub version_name: &'a str,
    pub version_protocol: u32,
    pub max_players: u32,
    pub online_players: u32,
    //pub player_samples: Vec<PlayerSample<'a>>,
    pub description: &'a str,
    pub favicon: &'a str,
}

impl<'a> StatusResponse<'a> {
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        let response_json = json!({
            "version": {
                "name": self.version_name,
                "protocol": self.version_protocol,
            },
            "players": {
                "max": self.max_players,
                "online": self.online_players,
            },
            "description": {
                "text": self.description,
            },
            "favicon": self.favicon,
            "enforcesSecureChat": false,
        });
        let response_json = response_json.to_string();

        (response_json.len() as i32).to_var_int(writer).await?;
        writer.write(response_json.as_bytes()).await?;
        writer.flush().await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct PingRequest {
    pub timestamp: u64,
}

impl PingRequest {
    pub async fn read_from<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Self> {
        let timestamp = reader.read_u64().await?;

        Ok(PingRequest {
            timestamp,
        })
    }
}

#[derive(Debug)]
pub struct PingResponse {
    pub timestamp: u64,
}

impl PingResponse {
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u64(self.timestamp).await
    }
}
