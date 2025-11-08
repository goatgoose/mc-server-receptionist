use std::fmt::Formatter;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{SeqAccess, Visitor};

struct VarInt {
    pub value: i32,
}

impl Serialize for VarInt {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer
    {
        todo!()
    }
}

impl<'de> Deserialize<'de> for VarInt {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>
    {
        struct VarIntVisitor;

        impl<'de> Visitor<'de> for VarIntVisitor {
            type Value = VarInt;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                formatter.write_str("Var variable-length integer")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let segment_bits = 0b01111111;
                let continue_bit = 0b10000000;

                let mut value: i32 = 0;
                let mut position = 0;

                loop {
                    let byte: u8 = seq.next_element()?
                        .ok_or_else(|| de::Error::custom("unexpected enf of VarInt"))?;
                    value |= ((byte & segment_bits) as i32) << position;

                    if byte & continue_bit == 0 {
                        break;
                    }

                    position += 7;

                    if position >= 35 {
                        return Err(de::Error::custom("VarInt is too big"));
                    }
                }

                Ok(VarInt {value})
            }
        }

        deserializer.deserialize_seq(VarIntVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn var_int() {
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
            let var_int: VarInt = serde::Deserialize::deserialize(
                de::value::SeqDeserializer::<_, de::value::Error>::new(
                    test_case.sample.into_iter()
                )
            ).unwrap();

            assert_eq!(var_int.value, test_case.expected_value);
        }
    }
}
