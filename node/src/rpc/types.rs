use jsonrpsee::core::Serialize;
use serde::Deserialize;

#[derive(Clone, Serialize, Deserialize)]
pub struct GenerateKeyResponse {
    pub id: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SignResponse {
    //pub operation: String,
    pub signature: String,
}
