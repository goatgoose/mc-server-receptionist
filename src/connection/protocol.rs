use crate::connection::codec::{VarInt, VarIntString};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

mod handshake;
mod login;
mod packet;
mod status;

pub use handshake::{Handshake, HandshakeIntent, Message};
pub use login::{
    EncryptionRequest, EncryptionResponse, LoginAcknowledged, LoginStart, LoginSuccess,
};
pub use packet::Packet;
pub use status::{PingRequest, PingResponse, StatusRequest, StatusResponse};
