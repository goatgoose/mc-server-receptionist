use crate::connection::codec::VarInt;
use crate::connection::protocol::Handshake;
use crate::connection::protocol::{HandshakeIntent, Message, PingRequest, StatusRequest};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub struct Packet<'a> {
    pub message: Message<'a>,
}

impl<'a> Packet<'a> {
    pub fn new(message: Message<'a>) -> Self {
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
                0x00 => Message::Handshake(Handshake::read_from(reader).await?),
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("Unknown packet ID: {:x}", id),
                    ));
                }
            },
            Some(HandshakeIntent::Status) => match id {
                0x00 => Message::StatusRequest(StatusRequest {}),
                0x01 => Message::PingRequest(PingRequest::read_from(reader).await?),
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        "Unrecognized status packet received",
                    ));
                }
            },
            Some(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "Unsupported connection path",
                ));
            }
        };

        Ok(Packet { message })
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
