use crate::connection::codec::{VarInt, VarIntString};
use crate::connection::protocol::{
    EncryptionRequest, EncryptionResponse, LoginAcknowledged, LoginStart, LoginSuccess,
    PingRequest, PingResponse, StatusRequest, StatusResponse,
};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt};

#[derive(Debug)]
pub enum Message {
    Handshake(Handshake),
    StatusRequest(StatusRequest),
    StatusResponse(StatusResponse),
    PingRequest(PingRequest),
    PingResponse(PingResponse),
    LoginStart(LoginStart),
    EncryptionRequest(EncryptionRequest),
    EncryptionResponse(EncryptionResponse),
    LoginSuccess(LoginSuccess),
    LoginAcknowledged(LoginAcknowledged),
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
