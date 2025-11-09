use std::io;
use std::io::Read;
use serde::{Deserialize, Serialize};
use crate::codec::VarInt;

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
                return Err(io::Error::new(io::ErrorKind::Unsupported, "Unknown packet ID"));
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
}

impl Handshake {
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Self> {
        let protocol_version = i32::from_var_int(reader)?;

        Ok(Handshake {
            protocol_version,
        })
    }
}
