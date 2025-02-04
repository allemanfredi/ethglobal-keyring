use crate::types::{
    channels::RpcCommands,
    keys::{InstanceKeyData, KeyType},
    operation::Operation,
};

use super::types::{GenerateKeyResponse, SignResponse};
use futures::channel::{mpsc, oneshot};
use jsonrpsee::{core::async_trait, proc_macros::rpc, types::ErrorObjectOwned};

#[rpc(client, server, namespace = "keyring")]
pub trait Rpc {
    #[method(name = "generateKey")]
    async fn generate_key(
        &self,
        signing_key_type: KeyType,
        instance_key_data: InstanceKeyData,
    ) -> Result<GenerateKeyResponse, ErrorObjectOwned>;

    #[method(name = "sign")]
    async fn sign(
        &self,
        id: String,
        operation: Operation,
        operation_signature: String,
    ) -> Result<SignResponse, ErrorObjectOwned>;
}

pub struct Methods;

#[async_trait]
impl RpcServer for Methods {
    async fn generate_key(
        &self,
        signing_key_type: KeyType,
        instance_key_data: InstanceKeyData,
    ) -> Result<GenerateKeyResponse, ErrorObjectOwned> {
        match signing_key_type {
            KeyType::Secp256k1 => {}
            _ => {
                tracing::error!("Invalid key type: {:?}", signing_key_type);
                return Err(ErrorObjectOwned::owned(400, "Invalid key type", Some(())));
            }
        };

        return Ok(GenerateKeyResponse {
            id: "1".to_string(),
        });
    }

    async fn sign(
        &self,
        id: String,
        operation: Operation,
        operation_signature: String,
    ) -> Result<SignResponse, ErrorObjectOwned> {
        return Ok(SignResponse { signature: "1".to_string() });
    }
}
