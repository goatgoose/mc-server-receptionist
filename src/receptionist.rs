use crate::codec::VarInt;
use crate::protocol::{Handshake, HandshakeIntent, Message, Packet};
use serde::de;
use std::io::Cursor;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};

pub struct Receptionist {}

impl Receptionist {
    pub async fn listen(self, addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on: {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            println!("Accepted connection from: {}", &addr);

            tokio::spawn(async move {
                let packet = Receptionist::read_packet(&stream, None).await;
                let Ok(packet) = packet else {
                    eprintln!("{}", packet.unwrap_err());
                    return;
                };

                let Message::Handshake(handshake) = packet.message else {
                    eprintln!("unexpected initial message");
                    return;
                };

                match handshake.intent {
                    HandshakeIntent::Status => {
                        let packet = Receptionist::read_packet(&stream, Some(HandshakeIntent::Status)).await;
                        let Ok(packet) = packet else {
                            eprintln!("{}", packet.unwrap_err());
                            return;
                        };

                        let Message::StatusRequest(status_request) = packet.message else {
                            eprintln!("Expected status request message");
                            return;
                        };
                    },
                    _ => {},
                }
            });
        }
    }

    async fn read_packet(stream: &TcpStream, connection_path: Option<HandshakeIntent>) -> Result<Packet, io::Error> {
        let mut buf = vec![0; 1024];
        loop {
            stream.readable().await?;
            match stream.try_read(&mut buf) {
                Ok(n) => {
                    buf.truncate(n);
                    println!("{:?}", buf);
                    break;
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        let mut cursor = Cursor::new(buf);
        let packet = Packet::read_from(&mut cursor, connection_path);
        println!("{:?}", packet);

        packet
    }
}
