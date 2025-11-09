use byteorder::ReadBytesExt;
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub trait VarInt {
    const MAX_BYTES: u8;
    async fn from_var_int<R: AsyncRead + Unpin>(reader: &mut R) -> Result<i32, io::Error>;
    async fn to_var_int<W: AsyncWrite + Unpin>(self, writer: &mut W) -> Result<(), io::Error>;
}

impl VarInt for i32 {
    const MAX_BYTES: u8 = 5;

    async fn from_var_int<R: AsyncRead + Unpin>(reader: &mut R) -> Result<i32, io::Error> {
        let section_bits = 0b01111111;
        let continue_bit = 0b10000000;

        let mut value = 0;
        let mut position = 0;
        loop {
            let byte = reader.read_u8().await?;
            value |= ((byte & section_bits) as i32) << position;

            if byte & continue_bit == 0 {
                break;
            }

            position += 7;
            if (position >= 7 * Self::MAX_BYTES) {
                return Err(io::Error::new(io::ErrorKind::InvalidData, "VarInt too big"));
            }
        }

        Ok(value)
    }

    async fn to_var_int<W: AsyncWrite + Unpin>(self, writer: &mut W) -> Result<(), io::Error> {
        let section_bits = 0b01111111;
        let continue_bit = 0b10000000;

        let mut value = self as u32;
        loop {
            let byte = (value & 0xFF) as u8;
            if value & !(section_bits as u32) == 0 {
                writer.write_u8(byte).await?;
                return Ok(())
            }

            let byte = (byte & section_bits) | continue_bit;
            writer.write_u8(byte).await?;

            value >>= 7;
        }
    }
}

pub trait VarIntString {
    async fn from_var_int_string<R: AsyncRead + Unpin>(reader: &mut R)
    -> Result<String, io::Error>;
}

impl VarIntString for String {
    async fn from_var_int_string<R: AsyncRead + Unpin>(
        reader: &mut R,
    ) -> Result<String, io::Error> {
        let len = i32::from_var_int(reader).await?;
        let mut buf: Vec<u8> = vec![0; len as usize];
        reader.read_exact(&mut buf).await?;
        let str = String::from_utf8(buf)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "String is not valid UTF8"))?;
        Ok(str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn var_int() -> Result<(), io::Error> {
        #[derive(Clone)]
        struct TestCase {
            sample: Vec<u8>,
            value: i32,
        }

        // test values from
        // https://minecraft.wiki/w/Java_Edition_protocol/Packets#VarInt_and_VarLong
        let test_cases = [
            TestCase {
                sample: vec![0x00],
                value: 0,
            },
            TestCase {
                sample: vec![0x01],
                value: 1,
            },
            TestCase {
                sample: vec![0x02],
                value: 2,
            },
            TestCase {
                sample: vec![0x7f],
                value: 127,
            },
            TestCase {
                sample: vec![0x80, 0x01],
                value: 128,
            },
            TestCase {
                sample: vec![0xff, 0x01],
                value: 255,
            },
            TestCase {
                sample: vec![0xdd, 0xc7, 0x01],
                value: 25565,
            },
            TestCase {
                sample: vec![0xff, 0xff, 0x7f],
                value: 2097151,
            },
            TestCase {
                sample: vec![0xff, 0xff, 0xff, 0xff, 0x07],
                value: 2147483647,
            },
            TestCase {
                sample: vec![0xff, 0xff, 0xff, 0xff, 0x0f],
                value: -1,
            },
            TestCase {
                sample: vec![0x80, 0x80, 0x80, 0x80, 0x08],
                value: -2147483648,
            },
        ];

        for test_case in test_cases.clone() {
            let mut cursor = Cursor::new(test_case.sample);
            let value = i32::from_var_int(&mut cursor).await?;

            assert_eq!(value, test_case.value);
        }

        for test_case in test_cases.clone() {
            let buf = Vec::<u8>::new();
            let mut cursor = Cursor::new(buf);
            test_case.value.to_var_int(&mut cursor).await?;

            assert_eq!(cursor.into_inner(), test_case.sample);
        }

        Ok(())
    }
}
