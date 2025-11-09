mod codec;
mod protocol;

use crate::connection::protocol::{
    EncryptionRequest, EncryptionResponse, Handshake, HandshakeIntent, LoginStart, Message, Packet,
    PingRequest, PingResponse, StatusRequest, StatusResponse,
};
use crate::util::AsyncPeek;
use std::collections::VecDeque;
use rand::Rng;
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::EncodePublicKey;
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

pub struct Connection<'a, S: AsyncRead + AsyncWrite + AsyncPeek + Unpin> {
    stream: S,
    send_queue: VecDeque<Packet<'a>>,
    path: Option<HandshakeIntent>,
    crypto: Crypto,
}

impl<'a, S: AsyncRead + AsyncWrite + AsyncPeek + Unpin> Connection<'a, S> {
    pub fn new(stream: S) -> Self {
        Connection {
            stream,
            send_queue: VecDeque::new(),
            path: None,
            crypto: Crypto::new(),
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
                Message::LoginStart(login_start) => self.recv_login_start(login_start)?,
                Message::EncryptionResponse(response) => self.recv_encryption_response(response)?,
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
        self.path = Some(handshake.intent);
        Ok(())
    }

    fn recv_status_request(&mut self, status_request: StatusRequest) -> Result<(), io::Error> {
        let response = StatusResponse {
            version_name: "1.21.10",
            version_protocol: 773,
            max_players: 20,
            online_players: 0,
            description: "A fake MC server!",
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

    fn recv_login_start(&mut self, login_start: LoginStart) -> Result<(), io::Error> {
        let public_key = self.crypto.public_key.to_public_key_der().unwrap();

        let mut rng = rand::thread_rng();
        let mut verify_token = [0u8; 16];
        rng.fill(&mut verify_token);

        let request = EncryptionRequest {
            sever_id: "".to_string(),
            public_key: public_key.into_vec(),
            verify_token: Vec::from(verify_token),
            should_authenticate: true,
        };
        let packet = Packet::new(Message::EncryptionRequest(request));
        self.send_queue.push_back(packet);
        Ok(())
    }

    fn recv_encryption_response(&mut self, response: EncryptionResponse) -> io::Result<()> {
        Ok(())
    }
}

struct Crypto {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
}

impl Crypto {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 1024)
            .expect("Failed to generate a key.");
        let public_key = RsaPublicKey::from(&private_key);

        Crypto {
            private_key,
            public_key,
        }
    }
}
