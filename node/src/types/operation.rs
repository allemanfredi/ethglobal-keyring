use crate::utils::from_hex_string;

use borsh_derive::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum Protocol {
    Evm,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, PartialEq, Debug)]
pub struct Operation {
    pub protocol: Protocol,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    #[serde(with = "from_hex_string")]
    #[serde(rename = "targetAddress")]
    pub target_address: Vec<u8>,
    #[serde(with = "from_hex_string")]
    pub data: Vec<u8>,
    #[serde(with = "from_hex_string")]
    pub salt: Vec<u8>,
}

impl Operation {
    pub fn to_borsh_encoded_vec(&self) -> Result<Vec<u8>, std::io::Error> {
        borsh::to_vec(&self)
    }
}
