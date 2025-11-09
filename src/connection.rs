use crate::protocol::{
    Handshake, HandshakeIntent, Message, MessageType, Packet, PingRequest, PingResponse,
    StatusRequest, StatusResponse,
};
use crate::util::AsyncPeek;
use std::collections::VecDeque;
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};
use tokio::net::TcpStream;

pub struct Connection<'a, S: AsyncRead + AsyncWrite + AsyncPeek + Unpin> {
    stream: S,
    send_queue: VecDeque<Packet<'a>>,
    path: Option<HandshakeIntent>,
}

impl<'a, S: AsyncRead + AsyncWrite + AsyncPeek + Unpin> Connection<'a, S> {
    pub fn new(stream: S) -> Self {
        Connection {
            stream,
            send_queue: VecDeque::new(),
            path: None,
        }
    }

    pub async fn process(&mut self) -> Result<(), io::Error> {
        loop {
            while let Some(packet) = self.send_queue.pop_front() {
                println!("sending: {:?}", packet);
                packet.write_to(&mut self.stream).await?;
            }

            // For now, always read when there's nothing to write. This works because the server
            // currently is always responding to something. After the server needs to send something
            // unprompted to the client, there should be a separate read and write task with a write
            // packet queue.
            let mut buf = vec![0, 1];
            match self.stream.peek(&mut buf).await {
                Ok(n) if n == 0 => break,
                Ok(_) => {}
                Err(e) => return Err(e),
            }

            let packet = Packet::read_from(&mut self.stream, self.path).await?;
            println!("received: {:?}", packet);

            match packet.message {
                Message::Handshake(handshake) => self.recv_handshake(handshake)?,
                Message::StatusRequest(request) => self.recv_status_request(request)?,
                Message::PingRequest(request) => self.recv_ping_request(request)?,
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
        let response = StatusResponse {
            version_name: "1.21.10",
            version_protocol: 773,
            max_players: 20,
            online_players: 0,
            description: "A test message!",
            favicon: "",
        };
        let packet = Packet::new(Message::StatusResponse(response));
        self.send_queue.push_back(packet);
        Ok(())
    }

    fn recv_ping_request(&mut self, ping_request: PingRequest) -> Result<(), io::Error> {
        let response = PingResponse {
            timestamp: ping_request.timestamp,
        };
        let packet = Packet::new(Message::PingResponse(response));
        self.send_queue.push_back(packet);
        Ok(())
    }
}
