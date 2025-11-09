use crate::protocol::{Handshake, HandshakeIntent, Message, MessageType, Packet};
use std::io::Cursor;
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::TcpStream;

pub struct Connection<S: AsyncRead + AsyncWrite + Unpin> {
    stream: S,
    next_read: bool,
    next_write: Option<MessageType>,
    path: Option<HandshakeIntent>,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Connection<S> {
    pub fn new(stream: S) -> Self {
        Connection {
            stream,
            next_read: true,
            next_write: None,
            path: None,
        }
    }

    pub async fn process(&mut self) -> Result<(), io::Error> {
        while !self.finished() {
            if self.next_read {
                self.next_read = false;

                let packet = Packet::read_from(&mut self.stream, self.path).await?;
                println!("{:?}", packet);

                match packet.message {
                    Message::Handshake(handshake) => self.recv_handshake(handshake)?,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            format!("Unimplemented message type: {:?}", packet.message),
                        ));
                    }
                }
            }

            if let Some(next_write) = &self.next_write {
                panic!("write side unimplemented");
            }
        }

        Ok(())
    }

    fn finished(&self) -> bool {
        if !self.next_read && let None = self.next_write {
            true
        } else {
            false
        }
    }

    fn recv_handshake(&mut self, handshake: Handshake) -> Result<(), io::Error> {
        match handshake.intent {
            HandshakeIntent::Status => {
                self.next_read = true;
                self.path = Some(handshake.intent);
            }
            _ => {}
        }
        Ok(())
    }
}
