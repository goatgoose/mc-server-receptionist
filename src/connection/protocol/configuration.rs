use crate::connection::codec::{VarInt, VarIntString};
use std::io;
use tokio::io::{AsyncWrite, AsyncWriteExt};

#[derive(Debug, Clone)]
pub struct Transfer {
    pub hostname: String,
    pub port: u16,
}

impl Transfer {
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        self.hostname.to_var_int_string(writer).await?;
        (self.port as i32).to_var_int(writer).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ClientboundKeepAlive {
    pub keep_alive_id: i64,
}

impl ClientboundKeepAlive {
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_i64(self.keep_alive_id).await
    }
}
