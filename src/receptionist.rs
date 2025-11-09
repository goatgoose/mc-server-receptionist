use std::io::Cursor;
use serde::de;
use tokio::net::TcpListener;
use tokio::io;
use crate::codec::VarInt;
use crate::protocol::{Handshake, Packet};

pub struct Receptionist {

}

impl Receptionist {
    pub async fn listen(self, addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        println!("Listening on: {}", addr);

        loop {
            let (stream, addr) = listener.accept().await?;
            println!("Accepted connection from: {}", &addr);

            tokio::spawn(async move {
                let mut buf = vec![0; 1024];
                loop {
                    stream.readable().await.unwrap();
                    match stream.try_read(&mut buf) {
                        Ok(n) => {
                            buf.truncate(n);
                            println!("{:?}", buf);
                            break;
                        },
                        Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                            continue;
                        },
                        Err(e) => {
                            eprintln!("{:?}", e);
                            return;
                        }
                    }
                }

                let mut cursor = Cursor::new(buf);
                let handshake = Packet::read_from(&mut cursor);
                println!("{:?}", handshake);
            });
        }
    }
}
