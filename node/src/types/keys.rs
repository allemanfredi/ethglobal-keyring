use crate::utils::from_hex_string;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum KeyType {
    Secp256k1,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct InstanceKeyData {
    #[serde(rename = "keyType")]
    pub key_type: KeyType,
    #[serde(with = "from_hex_string")]
    #[serde(rename = "publicKey")]
    pub public_key: Vec<u8>,
}

pub struct SigningKeyData {
    pub key_type: KeyType,
    pub private_key: Vec<u8>,
}
