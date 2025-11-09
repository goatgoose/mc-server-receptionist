use std::{
    io,
    io::Read,
};
use byteorder::ReadBytesExt;

pub trait VarInt {
    const MAX_BYTES: u8;
    fn from_var_int<R: Read>(reader: &mut R) -> Result<i32, io::Error>;
}

impl VarInt for i32 {
    const MAX_BYTES: u8 = 5;

    fn from_var_int<R: Read>(reader: &mut R) -> Result<i32, io::Error> {
        let section_bits = 0b01111111;
        let continue_bit = 0b10000000;

        let mut value = 0;
        let mut position = 0;
        loop {
            let byte = reader.read_u8()?;
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
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use super::*;

    #[test]
    fn var_int() -> Result<(), io::Error> {
        struct TestCase {
            sample: Vec<u8>,
            expected_value: i32,
        }

        // test values from
        // https://minecraft.wiki/w/Java_Edition_protocol/Packets#VarInt_and_VarLong
        let test_cases = [
            TestCase {
                sample: vec![0x00],
                expected_value: 0,
            },
            TestCase {
                sample: vec![0x01],
                expected_value: 1,
            },
            TestCase {
                sample: vec![0x02],
                expected_value: 2,
            },
            TestCase {
                sample: vec![0x7f],
                expected_value: 127,
            },
            TestCase {
                sample: vec![0x80, 0x01],
                expected_value: 128,
            },
            TestCase {
                sample: vec![0xff, 0x01],
                expected_value: 255,
            },
            TestCase {
                sample: vec![0xdd, 0xc7, 0x01],
                expected_value: 25565,
            },
            TestCase {
                sample: vec![0xff, 0xff, 0x7f],
                expected_value: 2097151,
            },
            TestCase {
                sample: vec![0xff, 0xff, 0xff, 0xff, 0x07],
                expected_value: 2147483647,
            },
            TestCase {
                sample: vec![0xff, 0xff, 0xff, 0xff, 0x0f],
                expected_value: -1,
            },
            TestCase {
                sample: vec![0x80, 0x80, 0x80, 0x80, 0x08],
                expected_value: -2147483648,
            },
        ];

        for test_case in test_cases {
            let mut cursor = Cursor::new(test_case.sample);
            let value = i32::from_var_int(&mut cursor)?;

            assert_eq!(value, test_case.expected_value);
        }

        Ok(())
    }
}
