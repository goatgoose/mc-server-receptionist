use crate::connection::codec::VarInt;
use crate::connection::protocol::{EncryptionResponse, Handshake, LoginAcknowledged, LoginStart};
use crate::connection::protocol::{HandshakeIntent, Message, PingRequest, StatusRequest};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct Packet {
    pub message: Message,
}

impl Packet {
    pub fn new(message: Message) -> Self {
        Packet { message }
    }

    pub async fn read_from<R: AsyncRead + Unpin>(
        reader: &mut R,
        connection_path: Option<HandshakeIntent>,
    ) -> io::Result<Self> {
        let length = i32::from_var_int(reader).await?;
        let id = i32::from_var_int(reader).await?;

        let message = match connection_path {
            None => match id {
                0x00 => Some(Message::Handshake(Handshake::read_from(reader).await?)),
                _ => None,
            },
            Some(HandshakeIntent::Status) => match id {
                0x00 => Some(Message::StatusRequest(StatusRequest {})),
                0x01 => Some(Message::PingRequest(PingRequest::read_from(reader).await?)),
                _ => None,
            },
            Some(HandshakeIntent::Login) => match id {
                0x00 => Some(Message::LoginStart(LoginStart::read_from(reader).await?)),
                0x01 => Some(Message::EncryptionResponse(
                    EncryptionResponse::read_from(reader).await?,
                )),
                0x03 => Some(Message::LoginAcknowledged(LoginAcknowledged {})),
                _ => None,
            },
            Some(_) => None,
        };

        match message {
            Some(message) => Ok(Packet { message }),
            None => {
                let mut unrecognized_bytes = vec![0; length as usize];
                reader.read_exact(&mut unrecognized_bytes).await?;

                Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!(
                        "Unrecognized packet received: {:X} for path {:?}",
                        id, connection_path
                    ),
                ))
            }
        }
    }

    pub async fn write_to<W: AsyncWrite + Unpin>(&self, writer: &mut W) -> io::Result<()> {
        let mut buf = Vec::new();

        match &self.message {
            Message::StatusResponse(response) => {
                let packet_id = 0x00;
                packet_id.to_var_int(&mut buf).await?;
                response.write_to(&mut buf).await?;
            }
            Message::PingResponse(response) => {
                let packet_id = 0x01;
                packet_id.to_var_int(&mut buf).await?;
                response.write_to(&mut buf).await?;
            }
            Message::EncryptionRequest(request) => {
                let packet_id = 0x01;
                packet_id.to_var_int(&mut buf).await?;
                request.write_to(&mut buf).await?;
            }
            Message::LoginSuccess(login_success) => {
                let packet_id = 0x02;
                packet_id.to_var_int(&mut buf).await?;
                login_success.write_to(&mut buf).await?;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Unimplemented message write",
                ));
            }
        }

        (buf.len() as i32).to_var_int(writer).await?;
        writer.write(buf.as_slice()).await?;

        writer.flush().await?;

        Ok(())
    }
}
