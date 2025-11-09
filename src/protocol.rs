use crate::codec::{VarInt, VarIntString};
use serde::{Deserialize, Serialize};
use std::io;
use std::io::Read;
use byteorder::{NetworkEndian, ReadBytesExt};

#[derive(Debug)]
pub struct Packet {
    pub length: i32,
    pub id: i32,
    pub message: Message,
}

impl Packet {
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let length = i32::from_var_int(reader)?;
        let id = i32::from_var_int(reader)?;

        let message = match id {
            0x00 => Message::Handshake(Handshake::read_from(reader)?),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    format!("Unknown packet ID: {:x}", id),
                ));
            }
        };

        Ok(Packet {
            length,
            id,
            message,
        })
    }
}

#[derive(Debug)]
pub enum Message {
    Handshake(Handshake),
}

#[derive(Debug)]
pub struct Handshake {
    pub protocol_version: i32,
    pub server_address: String,
    pub server_port: u16,
    pub intent: HandshakeIntent,
}

impl Handshake {
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let protocol_version = i32::from_var_int(reader)?;
        let server_address = String::from_var_int_string(reader)?;
        let server_port = reader.read_u16::<NetworkEndian>()?;
        let intent = HandshakeIntent::read_from(reader)?;

        Ok(Handshake {
            protocol_version,
            server_address,
            server_port,
            intent,
        })
    }
}

#[derive(Debug)]
pub enum HandshakeIntent {
    Status,
    Login,
    Transfer,
}

impl HandshakeIntent {
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let intent = i32::from_var_int(reader)?;
        match intent {
            1 => Ok(HandshakeIntent::Status),
            2 => Ok(HandshakeIntent::Login),
            3 => Ok(HandshakeIntent::Transfer),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown handshake intent")),
        }
    }
}
