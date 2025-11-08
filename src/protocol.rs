use std::io;
use std::io::Read;
use serde::{Deserialize, Serialize};
use crate::codec::VarInt;

pub struct Handshake {
    protocol_version: VarInt,
}

impl Handshake {
    pub fn read_from<R: Read>(reader: R) -> io::Result<Self> {
        protocol_version = VarInt::deserialize()
    }
}
