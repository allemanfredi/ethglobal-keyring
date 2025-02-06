use crate::{
    mpc::mpc_service::MpcServiceError,
    types::{
        channels::RpcCommands,
        keys::InstanceKeyData,
        operation::Operation,
        rpc::{GenerateKeyResponse, SignResponse},
    },
};
use alloy_signer::utils::public_key_to_address;
use futures::channel::{mpsc, oneshot};
use jsonrpsee::{core::async_trait, proc_macros::rpc, types::ErrorObjectOwned};
use k256::ecdsa::VerifyingKey;

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
        // TODO: add user authentication: User for example uses a passkey to ask
        // for a key generation from the Keyring network. When an user wants to sign something,
        // he must provide that key in order to trigger the signing process

        // TODO: validate instance_key_data

        let (tx, rx) = oneshot::channel();
        self.rpc_channel_tx
            .unbounded_send(RpcCommands::StartGenerateKey { response_tx: tx })
            .unwrap();
        match rx.await {
            Ok(result) => match result {
                Ok(shared_public_key) => {
                    // TODO: move this information within another api, for example, keyring_getAddressBySharedPublicKey
                    let vk =
                        VerifyingKey::from_sec1_bytes(&hex::decode(&shared_public_key).unwrap())
                            .unwrap();
                    let address = public_key_to_address(&vk);

                    return Ok(GenerateKeyResponse {
                        shared_public_key: format!("0x{shared_public_key}"),
                        shared_evm_address: Some(address.to_string()),
                    });
                }
                Err(err) => {
                    tracing::error!("failed to create the key. reason: {err}");
                    return Err(ErrorObjectOwned::owned(
                        1000,
                        "failed to create the key. reason: unknown",
                        Some(()),
                    ));
                }
            },
            Err(err) => {
                tracing::error!("failed to create the key. reason: {err}");
                return Err(ErrorObjectOwned::owned(
                    1001,
                    "failed to create the key",
                    Some(()),
                ));
            }
        };
    }

    async fn sign(
        &self,
        shared_public_key: String,
        operation: Operation,
        operation_signature: String,
    ) -> Result<SignResponse, ErrorObjectOwned> {
        let operation_borsh_encoded_vec = match operation.to_borsh_encoded_vec() {
            Ok(operation) => operation,
            Err(err) => {
                tracing::error!("failed to parse operation. reason: {err}");
                return Err(ErrorObjectOwned::owned(
                    2000,
                    "failed to parse operation",
                    Some(()),
                ));
            }
        };

        println!(
            "operation_borsh_encoded_vec {}",
            hex::encode(&operation_borsh_encoded_vec)
        );

        // TODO: call Gateway.canBeExecuted(operation_borsh_encoded_vec) to ensure that an user isn't using an operation
        // equal to an old one (salt didn't change)

        let (tx, rx) = oneshot::channel();
        self.rpc_channel_tx
            .unbounded_send(RpcCommands::StartSigning {
                response_tx: tx,
                shared_public_key,
                data: operation_borsh_encoded_vec,
            })
            .unwrap();
        match rx.await {
            Ok(result) => match result {
                Ok(signature) => {
                    return Ok(SignResponse { signature });
                }
                Err(err) => match err {
                    MpcServiceError::KeyShareNotFound(_) => {
                        tracing::error!("failed to sign. reason: key share not found");
                        return Err(ErrorObjectOwned::owned(
                            2001,
                            "failed to sign: reason: key share not found",
                            Some(()),
                        ));
                    }
                    _ => {
                        tracing::error!("failed to sign. reason: unknown");
                        return Err(ErrorObjectOwned::owned(
                            2002,
                            "failed to sign: reason: unknown",
                            Some(()),
                        ));
                    }
                },
            },
            Err(err) => {
                tracing::error!("failed to sign. reason: {err}");
                return Err(ErrorObjectOwned::owned(
                    2003,
                    "failed to create the signature. reason: unknown",
                    Some(()),
                ));
            }
        };
    }
}
