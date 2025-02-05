use jsonrpsee::core::Serialize;
use serde::Deserialize;

#[derive(Clone, Serialize, Deserialize)]
pub struct GenerateKeyResponse {
    #[serde(rename = "sharedPublicKey")]
    pub shared_public_key: String,
    #[serde(rename = "sharedEvmAddress")]
    pub shared_evm_address: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SignResponse {
    //pub operation: String,
    pub signature: String,
}
