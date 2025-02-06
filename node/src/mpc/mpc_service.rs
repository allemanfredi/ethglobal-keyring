use anyhow::Result;
use cggmp21::{
    key_refresh::AuxOnlyMsg,
    key_share::{DirtyAuxInfo, DirtyKeyShare, Valid},
    keygen::msg::threshold::Msg as KeygenMsg,
    round_based::{Incoming, MessageType},
    security_level::SecurityLevel128,
    signing::msg::Msg as SigningMessage,
    supported_curves::Secp256k1,
    PregeneratedPrimes,
};
use futures::{
    channel::{
        mpsc::{self, UnboundedSender},
        oneshot::{self, Sender},
    },
    lock::Mutex,
    StreamExt,
};
use k256::sha2::Sha256;
use rand::RngCore;
use rand_core::OsRng;
use std::{collections::HashMap, sync::Arc};

use crate::types::channels::{EventCommands, NetworkCommands, ProtocolEvents, RpcCommands};

use super::{
    aux_info_gen::{run_aux_info_gen, AuxInfoGenError},
    keygen::{run_keygen, KeygenError},
    signing::{run_signing, SigningErrors},
};

lazy_static! {
    static ref DB: Mutex<HashMap<String, Valid<DirtyKeyShare<Secp256k1, SecurityLevel128>>>> =
        Mutex::new(HashMap::new());
}

#[derive(Debug, thiserror::Error)]
pub enum MpcServiceError {
    #[error("failed to get the local signer id")]
    GetLocalSignerIdError,

    #[error("failed to get key share for shared_public_key={0}")]
    KeyShareNotFound(String),
}

pub struct MpcService {
    rpc_channel_rx: mpsc::UnboundedReceiver<RpcCommands>,
    network_channel_tx: mpsc::UnboundedSender<NetworkCommands>,
    network_channel_rx: mpsc::UnboundedReceiver<NetworkCommands>,
    protocol_event_channel_rx: mpsc::UnboundedReceiver<EventCommands>,
    active_channels_aux_info_gen: Arc<
        Mutex<
            HashMap<
                String,
                UnboundedSender<
                    Result<Incoming<AuxOnlyMsg<Sha256, SecurityLevel128>>, AuxInfoGenError>,
                >,
            >,
        >,
    >,
    active_channels_keygen: Arc<
        Mutex<
            HashMap<
                String,
                UnboundedSender<
                    Result<Incoming<KeygenMsg<Secp256k1, SecurityLevel128, Sha256>>, KeygenError>,
                >,
            >,
        >,
    >,
    active_channels_signing: Arc<
        Mutex<
            HashMap<
                String,
                UnboundedSender<Result<Incoming<SigningMessage<Secp256k1, Sha256>>, SigningErrors>>,
            >,
        >,
    >,
    pregenerated_primes: PregeneratedPrimes,
    aux_info: Arc<Mutex<Option<Valid<DirtyAuxInfo>>>>,
}

impl MpcService {
    pub fn new(
        rpc_channel_rx: mpsc::UnboundedReceiver<RpcCommands>,
        network_channel_tx: mpsc::UnboundedSender<NetworkCommands>,
        network_channel_rx: mpsc::UnboundedReceiver<NetworkCommands>,
        protocol_event_channel_rx: mpsc::UnboundedReceiver<EventCommands>,
        pregenerated_primes: PregeneratedPrimes,
    ) -> Self {
        MpcService {
            rpc_channel_rx,
            network_channel_tx,
            network_channel_rx,
            protocol_event_channel_rx,
            active_channels_keygen: Arc::new(Mutex::new(HashMap::new())),
            active_channels_signing: Arc::new(Mutex::new(HashMap::new())),
            active_channels_aux_info_gen: Arc::new(Mutex::new(HashMap::new())),
            pregenerated_primes,
            aux_info: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn start(&mut self) {
        loop {
            tokio::select! {
                // Handle messages from network service
                cmd = self.network_channel_rx.next() => {
                    match cmd {
                        Some(NetworkCommands::AllPeersJoined) => {
                            let local_signer_id =  self.get_local_signer_id().await.unwrap();
                            let eid = MpcService::generate_eid();
                            // NOTE: for now the signer with index = 0 is the one in charge of starting the auxiliary info generation
                            // NOTE: Considering in the initial phase the number of signers is fixed, we can generate the primes only once.
                            // check here: https://github.com/LFDT-Lockness/cggmp21/tree/5e621acd25aa492941ef9c4491a1c7aa16a39807?tab=readme-ov-file#on-reusability-of-the-auxiliary-data
                            if local_signer_id == 0 {
                                tracing::info!("starting generating auxiliary info with signer_id={local_signer_id} for eid={eid} ...");
                                self.run_aux_info_gen(eid, true).await;
                            }
                        },
                        _ => {
                            tracing::error!("received an invalid network command");
                        },
                    }
                },
                // Handle messages from network service
                cmd = self.protocol_event_channel_rx.next() => {
                    if let Some(EventCommands::NewEvent { data }) = cmd {
                        match bincode::deserialize(&data) {
                            Ok(ProtocolEvents::JoinAuxInfoGen { eid}) => {
                                tracing::info!("joining auxiliary info generation with signer_id={} for eid={eid} ...", self.get_local_signer_id().await.unwrap());
                                self.run_aux_info_gen(eid, false).await;
                            },
                            Ok(ProtocolEvents::JoinKeyGen { eid }) => {
                                tracing::info!("joining key generation with signer_id={} for eid={eid} ...", self.get_local_signer_id().await.unwrap());
                                self.run_keygen(eid, false, None).await;
                            },
                            Ok(ProtocolEvents::JoinSigning { eid, shared_public_key, data }) => {
                                tracing::info!("joining signing data={} with signer_id={} for eid={eid} and shared_public_key={shared_public_key} ...", hex::encode(&data), self.get_local_signer_id().await.unwrap());
                                self.run_signing(eid, data, shared_public_key, false, None).await;
                            },
                            Ok(ProtocolEvents::Cggmp21CoreAuxInfoGen { eid, message_id, data, signer_id, receiver_signer_id }) => {
                                let local_signer_id= self.get_local_signer_id().await.unwrap();
                                let msg_type = if let Some(to_signer_id) = receiver_signer_id {
                                    // TODO: be sure that the node knows the receiver_signer_id
                                    if to_signer_id != local_signer_id {
                                        continue;
                                    }
                                    MessageType::P2P
                                } else {
                                    MessageType::Broadcast
                                };

                                let active_channels_aux_info_gen_locked = self.active_channels_aux_info_gen.lock().await;
                                let tx = active_channels_aux_info_gen_locked.get(&eid).unwrap();

                                let incoming = Incoming {
                                    id: message_id,
                                    sender: signer_id,
                                    msg_type,
                                    msg: bincode::deserialize::<AuxOnlyMsg<Sha256, SecurityLevel128>>(&data).unwrap()
                                };

                                tx.unbounded_send(Ok(incoming))
                                .unwrap_or_else(|err| {
                                    tracing::error!("failed to send the message into the cggmp21 mpc stream: {err}");
                                });
                            },
                            Ok(ProtocolEvents::Cggmp21CoreKeygen { eid, message_id, data, signer_id, receiver_signer_id }) => {
                                let local_signer_id= self.get_local_signer_id().await.unwrap();
                                let msg_type = if let Some(to_signer_id) = receiver_signer_id {
                                    // TODO: be sure that the node knows the receiver_signer_id
                                    if to_signer_id != local_signer_id {
                                        continue;
                                    }
                                    MessageType::P2P
                                } else {
                                    MessageType::Broadcast
                                };

                                let active_channels_keygen_locked = self.active_channels_keygen.lock().await;
                                let tx = active_channels_keygen_locked.get(&eid).unwrap();

                                let incoming = Incoming {
                                    id: message_id,
                                    sender: signer_id,
                                    msg_type,
                                    msg: bincode::deserialize::<KeygenMsg<Secp256k1, SecurityLevel128, Sha256>>(&data).unwrap()
                                };
                                tx.unbounded_send(Ok(incoming))
                                .unwrap_or_else(|err| {
                                    tracing::error!("failed to send the message into the cggmp21 mpc stream: {err}");
                                });
                            },
                            Ok(ProtocolEvents::Cggmp21CoreSigning { eid, message_id, data, signer_id, receiver_signer_id }) => {
                                let local_signer_id= self.get_local_signer_id().await.unwrap();
                                let msg_type = if let Some(to_signer_id) = receiver_signer_id {
                                    // TODO: be sure that the node knows the receiver_signer_id
                                    if to_signer_id != local_signer_id {
                                        continue;
                                    }
                                    MessageType::P2P
                                } else {
                                    MessageType::Broadcast
                                };

                                let active_channels_signing_locked = self.active_channels_signing.lock().await;
                                let tx = active_channels_signing_locked.get(&eid).unwrap();

                                let incoming = Incoming {
                                    id: message_id,
                                    sender: signer_id,
                                    msg_type,
                                    msg: bincode::deserialize::<SigningMessage<Secp256k1, Sha256>>(&data).unwrap()
                                };

                                tx.unbounded_send(Ok(incoming))
                                .unwrap_or_else(|err| {
                                    tracing::error!("failed to send the message into the cggmp21 mpc stream: {err}");
                                });
                            },
                            Err(err) => {
                                tracing::error!("failed to deserialize the protocol event {}. Reason: {err}", hex::encode(data));
                            }
                        }

                    } else {
                        tracing::error!("Unknown event");
                    }

                },
                // Handle messages from rpc service
                cmd = self.rpc_channel_rx.next() => {
                    match cmd {
                        Some(RpcCommands::StartGenerateKey { response_tx }) => {
                            let eid = MpcService::generate_eid();
                            tracing::info!("starting generating key with signer_id={} for eid={eid} ...",  self.get_local_signer_id().await.unwrap());
                            self.run_keygen(eid.clone(), true, Some(response_tx)).await;
                        },
                        Some(RpcCommands::StartSigning { response_tx, shared_public_key, data }) => {
                            let eid = MpcService::generate_eid();
                            tracing::info!("starting signing data={} with signer_id={} for eid={eid} and shared_public_key={shared_public_key} ...", hex::encode(&data),  self.get_local_signer_id().await.unwrap());
                            self.run_signing(eid.clone(), data, shared_public_key, true, Some(response_tx)).await;
                        },
                        None => {
                            tracing::error!("invalid rpc command");
                        }
                    }
                }
            }
        }
    }

    async fn get_local_signer_id(&self) -> Result<u16> {
        let (tx, rx) = oneshot::channel();
        self.network_channel_tx
            .unbounded_send(NetworkCommands::GetLocalSignerId { response_tx: tx })
            .unwrap();
        let signer_id = match rx.await {
            Ok(signer_id) => signer_id,
            Err(_) => {
                tracing::error!("failed to retrieve the signer_id");
                return Err(MpcServiceError::GetLocalSignerIdError.into());
            }
        };
        Ok(signer_id)
    }

    async fn run_aux_info_gen(&mut self, eid: String, send_join_aux_info_gen_event: bool) {
        let pregenerated_primes = self.pregenerated_primes.clone();
        let network_channel_tx = self.network_channel_tx.clone();
        let active_channels_aux_info_gen = Arc::clone(&self.active_channels_aux_info_gen);
        let local_signer_id = self.get_local_signer_id().await.unwrap();

        // NOTE: we need to create a channel to receive the aux info and store them as we need them for key generation
        let (response_tx, response_rx) = oneshot::channel::<String>(); // Valid<DirtyAuxInfo> doesn't implement debug so it doesn't work
        let aux_info_mutex = Arc::clone(&self.aux_info);
        tokio::spawn(async move {
            match response_rx.await {
                Ok(res) => {
                    let mut aux_info = aux_info_mutex.lock().await;
                    *aux_info = Some(serde_json::from_str::<Valid<DirtyAuxInfo>>(&res).unwrap());
                    // Valid<DirtyAuxInfo> doesn't implement debug so it doesn't work
                }
                Err(_) => {
                    tracing::error!("failed to retrieve the auxiliary info");
                    // TODO: handle better errors
                    panic!("failed to retrieve the auxiliary info");
                }
            }
        });

        tokio::spawn(async move {
            let tx_0 = run_aux_info_gen(
                network_channel_tx,
                response_tx,
                send_join_aux_info_gen_event,
                local_signer_id,
                eid.clone(),
                pregenerated_primes,
            )
            .await
            .unwrap();
            let mut map_guard = active_channels_aux_info_gen.lock().await;
            map_guard.insert(eid, tx_0);
        });
    }

    pub async fn run_keygen(
        &mut self,
        eid: String,
        send_join_keygen_event: bool,
        rpc_response_tx: Option<Sender<Result<String, MpcServiceError>>>,
    ) {
        let network_channel_tx = self.network_channel_tx.clone();
        let active_channels_keygen = Arc::clone(&self.active_channels_keygen);
        let local_signer_id = self.get_local_signer_id().await.unwrap();
        let aux_info_mutex = Arc::clone(&self.aux_info);
        let (response_tx, response_rx) = oneshot::channel::<String>(); // Valid<DirtyAuxInfo> doesn't implement debug so it doesn't work

        tokio::spawn(async move {
            if let Some(aux_info) = aux_info_mutex.lock().await.as_ref() {
                let tx_0 = run_keygen(
                    network_channel_tx,
                    response_tx,
                    send_join_keygen_event,
                    eid.clone(),
                    local_signer_id,
                    aux_info.clone(),
                    3, // FIXME: read it on chain from the contract that handle registrations
                    3, // FIXME: must be equal to the number of participants
                )
                .await
                .unwrap();
                let mut map_guard = active_channels_keygen.lock().await;
                map_guard.insert(eid, tx_0);
            } else {
                tracing::error!("failed to retrieve the auxiliary info whithin key generation")
            }
        });

        tokio::spawn(async move {
            match response_rx.await {
                Ok(serialized_key_share) => {
                    let key_share = serde_json::from_str::<
                        Valid<DirtyKeyShare<Secp256k1, SecurityLevel128>>,
                    >(&serialized_key_share)
                    .unwrap();

                    let shared_public_key =
                        hex::encode(key_share.shared_public_key.to_bytes(true).to_vec());

                    // TODO: store result
                    DB.lock().await.insert(shared_public_key.clone(), key_share);

                    // NOTE: if run_keygen has been called from RPC, we need to return the result
                    if let Some(rpc_response_tx) = rpc_response_tx {
                        rpc_response_tx.send(Ok(shared_public_key)).unwrap();
                    }
                }
                Err(_) => {
                    tracing::error!("failed to read keygen result");
                }
            }
        });
    }

    pub async fn run_signing(
        &mut self,
        eid: String,
        data: Vec<u8>,
        shared_public_key: String,
        send_join_signing_event: bool,
        rpc_response_tx: Option<Sender<Result<String, MpcServiceError>>>,
    ) {
        // NOTE: To sign using a 2-out-of-3 scheme, the signers MUST be 0 and 1.
        // For example, if you need a 3-out-of-5 signature, the signers MUST be 0, 1, and 2.
        // https://github.com/LFDT-Lockness/cggmp21/blob/5e621acd25aa492941ef9c4491a1c7aa16a39807/cggmp21/src/signing.rs#L546
        // At the moment we force all participants to join the signature process.
        let network_channel_tx = self.network_channel_tx.clone();
        let active_channels_signing = Arc::clone(&self.active_channels_signing);
        let local_signer_id = self.get_local_signer_id().await.unwrap();
        let (response_tx, response_rx) = oneshot::channel::<String>(); // Valid<DirtyAuxInfo> doesn't implement debug so it doesn't work

        // NOTE: get the key share corresponding to the shared public key provided
        if let Some(key_share) = DB.lock().await.get(&shared_public_key).cloned() {
            tokio::spawn(async move {
                match run_signing(
                    network_channel_tx,
                    response_tx,
                    send_join_signing_event,
                    eid.clone(),
                    data,
                    local_signer_id,
                    key_share,
                    vec![0, 1, 2],
                )
                .await
                {
                    Ok(tx_0) => {
                        let mut map_guard = active_channels_signing.lock().await;
                        map_guard.insert(eid, tx_0);
                    }
                    Err(err) => {
                        tracing::error!("error during signature generation. reason: {err}");
                        // TODO
                        /*if let Some(rpc_response_tx) = rpc_response_tx {
                            rpc_response_tx
                                .send(Err(MpcServiceError::KeyShareNotFound(
                                    shared_public_key.clone(),
                                )))
                                .unwrap();
                        }*/
                    }
                }
            });

            tokio::spawn(async move {
                match response_rx.await {
                    Ok(serialized_signature) => {
                        // NOTE: if run_keygen has been called from RPC, we need to return the result
                        if let Some(rpc_response_tx) = rpc_response_tx {
                            rpc_response_tx.send(Ok(serialized_signature)).unwrap();
                        }
                    }
                    Err(err) => {
                        tracing::error!("failed to read the signature. reason: {err}");
                    }
                }
            });
        } else {
            tracing::error!("key share not found for shared_public_key={shared_public_key}");
            if let Some(rpc_response_tx) = rpc_response_tx {
                rpc_response_tx
                    .send(Err(MpcServiceError::KeyShareNotFound(
                        shared_public_key.clone(),
                    )))
                    .unwrap();
            }
        }
    }

    fn generate_eid() -> String {
        let mut random_bytes = [0u8; 32];
        OsRng.fill_bytes(&mut random_bytes);
        hex::encode(random_bytes)
    }
}
