use crate::connection::codec::{PrefixedArray, VarInt, VarIntString};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use uuid::Uuid;

#[derive(Debug)]
pub struct LoginStart {
    pub username: String,
    pub uuid: Uuid,
}

impl LoginStart {
    pub async fn read_from<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Self> {
        let username = String::from_var_int_string(reader).await?;
        let uuid = Uuid::from_u128(reader.read_u128().await?);

        Ok(LoginStart { username, uuid })
    }
}

#[derive(Debug)]
pub struct EncryptionRequest {
    pub sever_id: String,
    pub public_key: Vec<u8>,
    pub verify_token: Vec<u8>,
    pub should_authenticate: bool,
}

impl EncryptionRequest {
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        self.sever_id.to_var_int_string(writer).await?;
        self.public_key.to_prefixed_array(writer).await?;
        self.verify_token.to_prefixed_array(writer).await?;
        writer.write_u8(self.should_authenticate as u8).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct EncryptionResponse {
    pub shared_secret: Vec<u8>,
    pub verify_token: Vec<u8>,
}

impl EncryptionResponse {
    pub async fn read_from<R: AsyncRead + Unpin>(reader: &mut R) -> io::Result<Self> {
        let shared_secret = Vec::<u8>::from_prefixed_array(reader).await?;
        let verify_token = Vec::<u8>::from_prefixed_array(reader).await?;

        Ok(EncryptionResponse {
            shared_secret,
            verify_token,
        })
    }
}

#[derive(Debug)]
pub struct LoginSuccess {
    pub uuid: Uuid,
    pub username: String,
}

impl LoginSuccess {
    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_u128(self.uuid.as_u128()).await?;
        self.username.to_var_int_string(writer).await?;
        // empty properties array
        0.to_var_int(writer).await?;

        Ok(())
    }
}

#[derive(Debug)]
pub struct LoginAcknowledged {}
