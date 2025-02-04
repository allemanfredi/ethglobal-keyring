use hex;
use serde::de::{self, Deserializer};
use serde::ser::Serializer;
use serde::Deserialize;

pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let from_hex_string = hex::encode(bytes);
    serializer.serialize_str(&from_hex_string)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let from_hex_string: &str = Deserialize::deserialize(deserializer)?;
    hex::decode(from_hex_string).map_err(de::Error::custom)
}
