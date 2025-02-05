use crate::types::{
    channels::RpcCommands,
    keys::InstanceKeyData,
    operation::Operation,
    rpc::{GenerateKeyResponse, SignResponse},
};
use futures::channel::mpsc;
use jsonrpsee::{core::async_trait, proc_macros::rpc, types::ErrorObjectOwned};

#[rpc(client, server, namespace = "keyring")]
pub trait Rpc {
    #[method(name = "generateKey")]
    async fn generate_key(
        &self,
        instance_key_data: InstanceKeyData,
    ) -> Result<GenerateKeyResponse, ErrorObjectOwned>;

    #[method(name = "sign")]
    async fn sign(
        &self,
        shared_public_key: String,
        operation: Operation,
        operation_signature: String,
    ) -> Result<SignResponse, ErrorObjectOwned>;
}

pub struct Methods {
    pub rpc_channel_tx: mpsc::UnboundedSender<RpcCommands>,
}

#[async_trait]
impl RpcServer for Methods {
    async fn generate_key(
        &self,
        instance_key_data: InstanceKeyData,
    ) -> Result<GenerateKeyResponse, ErrorObjectOwned> {
        return Ok(GenerateKeyResponse {
            shared_public_key: "1".to_string(),
            shared_evm_address: Some("2".to_string()),
        });
    }

    async fn sign(
        &self,
        shared_public_key: String,
        operation: Operation,
        operation_signature: String,
    ) -> Result<SignResponse, ErrorObjectOwned> {
        return Ok(SignResponse {
            signature: "sig".to_string(),
        });
    }
}
