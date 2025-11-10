use crate::connection::codec::VarInt;
use serde_json::json;
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct StatusRequest {}

#[derive(Debug)]
pub struct StatusResponse {
    pub version_name: String,
    pub version_protocol: u32,
    pub max_players: u32,
    pub online_players: u32,
    //pub player_samples: Vec<PlayerSample<'a>>,
    pub description: String,
    pub favicon: String,
}

impl StatusResponse {
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

        Ok(PingRequest { timestamp })
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
