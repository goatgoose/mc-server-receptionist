use crate::connection::codec::{VarInt, VarIntString};
use std::io;
use tokio::io::AsyncWrite;

#[derive(Debug)]
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
