use crate::protocol::{Handshake, HandshakeIntent, Message, MessageType, Packet, StatusRequest};
use std::io::{Cursor, Error};
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::TcpStream;
use crate::protocol::MessageType::StatusResponse;
use crate::util::AsyncPeek;

pub struct Connection<S: AsyncRead + AsyncWrite + AsyncPeek+ Unpin> {
    stream: S,
    next_write: Option<MessageType>,
    path: Option<HandshakeIntent>,
}

impl<S: AsyncRead + AsyncWrite + AsyncPeek+ Unpin> Connection<S> {
    pub fn new(stream: S) -> Self {
        Connection {
            stream,
            next_write: None,
            path: None,
        }
    }

    pub async fn process(&mut self) -> Result<(), io::Error> {
        loop {
            if let Some(next_write) = &self.next_write {
                match next_write {
                    MessageType::StatusResponse => self.send_status_response()?,
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            format!("Unimplemented send message type: {:?}", next_write),
                        ));
                    }
                }
            }

            // For now, always read when there's nothing to write. This works because the server
            // currently is always responding to something. After the server needs to send something
            // unprompted to the client, there should be a separate read and write task with a write
            // packet queue.
            let mut buf = vec![0, 1];
            match self.stream.peek(&mut buf).await {
                Ok(n) if n == 0 => break,
                Ok(_) => {},
                Err(e) => return Err(e),
            }

            let packet = Packet::read_from(&mut self.stream, self.path).await?;
            println!("{:?}", packet);

            match packet.message {
                Message::Handshake(handshake) => self.recv_handshake(handshake)?,
                Message::StatusRequest(status_request) => self.recv_status_request(status_request)?,
                _ => {
                    return Err(io::Error::new(
                        io::ErrorKind::Unsupported,
                        format!("Unimplemented recv message type: {:?}", packet.message),
                    ));
                }
            }
        }

        Ok(())
    }

    fn recv_handshake(&mut self, handshake: Handshake) -> Result<(), io::Error> {
        match handshake.intent {
            HandshakeIntent::Status => {
                self.path = Some(handshake.intent);
            }
            _ => {}
        }
        Ok(())
    }

    fn recv_status_request(&mut self, status_request: StatusRequest) -> Result<(), io::Error> {
        self.next_write = Some(StatusResponse);
        Ok(())
    }

    fn send_status_response(&mut self) -> Result<(), io::Error> {
        Ok(())
    }
}
