mod codec;
mod protocol;

use crate::connection::protocol::{
    EncryptionRequest, EncryptionResponse, Handshake, HandshakeIntent, LoginAcknowledged,
    LoginStart, LoginSuccess, Message, Packet, PingRequest, PingResponse, StatusRequest,
    StatusResponse, Transfer,
};
use crate::util::AsyncPeek;
use aes::Aes128;
use cfb8::Cfb8;
use cfb8::cipher::{AsyncStreamCipher, NewCipher};
use rand::Rng;
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::pkcs8::EncodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
use std::collections::VecDeque;
use std::io::ErrorKind::InvalidData;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufWriter, ReadBuf};
use uuid::Uuid;

type AesCfb8 = Cfb8<Aes128>;

pub struct Connection<S: AsyncRead + AsyncWrite + AsyncPeek + Unpin> {
    stream: S,
    send_queue: VecDeque<Packet>,
    path: Option<HandshakeIntent>,
    crypto: Crypto,
    player_uuid: Option<Uuid>,
    player_username: Option<String>,
}

impl<S: AsyncRead + AsyncWrite + AsyncPeek + Unpin> Connection<S> {
    pub fn new(stream: S) -> Self {
        Connection {
            stream,
            send_queue: VecDeque::new(),
            path: None,
            crypto: Crypto::new(),
            player_uuid: None,
            player_username: None,
        }
    }

    pub async fn process(&mut self) -> Result<(), io::Error> {
        loop {
            while let Some(packet) = self.send_queue.pop_front() {
                println!("sending: {:?}", packet);

                let mut buf = Vec::new();
                packet.write_to(&mut buf).await?;

                if let Some(cipher) = &mut self.crypto.encrypt_cipher {
                    cipher.encrypt(buf.as_mut_slice());
                }

                self.stream.write_all(buf.as_slice()).await?;
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

            let packet = if let Some(cipher) = &mut self.crypto.decrypt_cipher {
                let mut reader = DecryptingReader::new(&mut self.stream, cipher);
                Packet::read_from(&mut reader, self.path).await
            } else {
                Packet::read_from(&mut self.stream, self.path).await
            };

            if let Err(e) = &packet
                && e.kind() == io::ErrorKind::Unsupported
            {
                eprintln!("{}", e);
                continue;
            }

            let packet = packet?;
            println!("received: {:?}", packet);

            match packet.message {
                Message::Handshake(handshake) => self.recv_handshake(handshake)?,
                Message::StatusRequest(request) => self.recv_status_request(request)?,
                Message::PingRequest(request) => self.recv_ping_request(request)?,
                Message::LoginStart(login_start) => self.recv_login_start(login_start)?,
                Message::EncryptionResponse(response) => self.recv_encryption_response(response)?,
                Message::LoginAcknowledged(ack) => self.recv_login_ack(ack)?,
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
            version_name: "1.21.10".to_string(),
            version_protocol: 773,
            max_players: 20,
            online_players: 0,
            description: "A fake MC server!".to_string(),
            favicon: "".to_string(),
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
        self.player_uuid = Some(login_start.uuid);
        self.player_username = Some(login_start.username);

        let public_key = self.crypto.public_key.to_public_key_der().unwrap();

        let request = EncryptionRequest {
            sever_id: "".to_string(),
            public_key: public_key.into_vec(),
            verify_token: self.crypto.verify_token.clone(),
            should_authenticate: true,
        };
        let packet = Packet::new(Message::EncryptionRequest(request));
        self.send_queue.push_back(packet);
        Ok(())
    }

    fn recv_encryption_response(&mut self, response: EncryptionResponse) -> io::Result<()> {
        let shared_secret = self
            .crypto
            .private_key
            .decrypt(Pkcs1v15Encrypt, response.shared_secret.as_slice())
            .map_err(|_| io::Error::new(InvalidData, "Unable to decrypt shared secret"))?;
        let verify_token = self
            .crypto
            .private_key
            .decrypt(Pkcs1v15Encrypt, response.verify_token.as_slice())
            .map_err(|_| io::Error::new(InvalidData, "Unable to decrypt verify token"))?;

        println!("shared secret: {:?}", shared_secret);
        println!("verify token: {:?}", verify_token);

        if verify_token != self.crypto.verify_token {
            return Err(io::Error::new(InvalidData, "invalid verify token"));
        }

        self.crypto.encrypt_cipher = Some(
            AesCfb8::new_from_slices(shared_secret.as_slice(), shared_secret.as_slice()).unwrap(),
        );
        self.crypto.decrypt_cipher = Some(
            AesCfb8::new_from_slices(shared_secret.as_slice(), shared_secret.as_slice()).unwrap(),
        );

        let login_success = LoginSuccess {
            uuid: self.player_uuid.unwrap(),
            username: self.player_username.clone().unwrap(),
        };
        let packet = Packet::new(Message::LoginSuccess(login_success));
        self.send_queue.push_back(packet);

        Ok(())
    }

    fn recv_login_ack(&mut self, ack: LoginAcknowledged) -> io::Result<()> {
        let transfer = Transfer {
            hostname: "classic.goatgoose.com".to_string(),
            port: 25565,
        };
        let packet = Packet::new(Message::Transfer(transfer));
        self.send_queue.push_back(packet);

        Ok(())
    }
}

struct Crypto {
    private_key: RsaPrivateKey,
    public_key: RsaPublicKey,
    verify_token: Vec<u8>,
    encrypt_cipher: Option<AesCfb8>,
    decrypt_cipher: Option<AesCfb8>,
}

impl Crypto {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let private_key = RsaPrivateKey::new(&mut rng, 1024).expect("Failed to generate a key.");
        let public_key = RsaPublicKey::from(&private_key);

        let mut verify_token = [0u8; 16];
        rng.fill(&mut verify_token);
        let verify_token = Vec::from(verify_token);

        Crypto {
            private_key,
            public_key,
            verify_token,
            encrypt_cipher: None,
            decrypt_cipher: None,
        }
    }
}

struct DecryptingReader<'a, R: AsyncRead + Unpin> {
    reader: &'a mut R,
    cipher: &'a mut AesCfb8,
}

impl<'a, R: AsyncRead + Unpin> DecryptingReader<'a, R> {
    pub fn new(reader: &'a mut R, cipher: &'a mut AesCfb8) -> Self {
        DecryptingReader { reader, cipher }
    }
}

impl<'a, R: AsyncRead + Unpin> AsyncRead for DecryptingReader<'a, R> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let before = buf.filled().len();
        let self_mut = self.get_mut();
        let result = Pin::new(&mut self_mut.reader).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = result {
            let after = buf.filled().len();
            if after > before {
                let filled = buf.filled_mut();
                self_mut.cipher.decrypt(&mut filled[before..after]);
            }
        }

        result
    }
}
