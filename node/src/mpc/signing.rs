use anyhow::Result;
use cggmp21::{
    key_share::{DirtyKeyShare, Valid},
    progress::PerfProfiler,
    round_based::{Incoming, MessageDestination, MpcParty, Outgoing},
    security_level::SecurityLevel128,
    signing::msg::Msg,
    supported_curves::Secp256k1,
    ExecutionId,
};
use futures::{
    channel::{
        mpsc::{self, UnboundedSender},
        oneshot,
    },
    StreamExt,
};
use k256::{
    ecdsa::{RecoveryId, Signature, VerifyingKey},
    sha2::{Digest, Sha256},
};
use rand::Rng;
use rand_core::OsRng;

use crate::types::channels::{NetworkCommands, ProtocolEvents};

#[derive(Debug, thiserror::Error)]
pub enum SigningErrors {
    #[error("Sign error")]
    SignError(#[from] cggmp21::signing::SigningError),
}

pub async fn run_signing(
    network_channel_tx: mpsc::UnboundedSender<NetworkCommands>,
    response_tx: oneshot::Sender<String>,
    send_join_signing_event: bool,
    eid: String,
    data: Vec<u8>,
    local_signer_id: u16,
    key_share: Valid<DirtyKeyShare<Secp256k1, SecurityLevel128>>,
    participant_signers_ids: Vec<u16>,
) -> Result<UnboundedSender<Result<Incoming<Msg<Secp256k1, Sha256>>, SigningErrors>>> {
    let (tx_0, rx_0) = mpsc::unbounded::<Result<Incoming<Msg<Secp256k1, Sha256>>, SigningErrors>>();
    let (tx_1, mut rx_1) = mpsc::unbounded::<Outgoing<Msg<Secp256k1, Sha256>>>();
    let party = MpcParty::connected((rx_0, tx_1));

    if send_join_signing_event {
        // NOTE: trigger node-keyring to call .sign().start()
        let event = ProtocolEvents::JoinSigning {
            eid: eid.clone(),
            shared_public_key: hex::encode(key_share.shared_public_key.to_bytes(true).to_vec()),
            data: data.clone(),
        };
        let event_bytes = bincode::serialize(&event).unwrap();
        network_channel_tx
            .unbounded_send(NetworkCommands::Send { data: event_bytes })
            .unwrap();
    }

    // NOTE: forward messages to swarm
    let eid_clone_for_signing = eid.clone();
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                outgoing = rx_1.next() => {
                    match outgoing {
                        Some(outgoing) => {
                            let event = ProtocolEvents::Cggmp21CoreSigning {
                                eid: eid.clone(),
                                message_id: OsRng.gen::<u64>(),
                                data: bincode::serialize(&outgoing.msg).unwrap(),
                                signer_id: local_signer_id,
                                receiver_signer_id: match outgoing.recipient {
                                    MessageDestination::OneParty(index) => Some(index),
                                    _ => None,
                                }
                            };
                            network_channel_tx.unbounded_send(NetworkCommands::Send { data: bincode::serialize(&event).unwrap() }).unwrap();
                        },
                        None => {
                            break;
                        }
                    }
                },
                _ = tokio::task::yield_now() => {}
            }
        }
    });

    tokio::spawn(async move {
        let signer_id = local_signer_id;

        let mut tracer = PerfProfiler::new();
        let data_hash = cggmp21::DataToSign::digest::<Sha256>(&data);
        match cggmp21::signing(
            ExecutionId::new(eid_clone_for_signing.as_bytes()),
            signer_id,
            &participant_signers_ids,
            &key_share,
        )
        .set_progress_tracer(&mut tracer)
        .sign(&mut OsRng, party, data_hash.clone())
        .await
        {
            Ok(signature) => {
                let v = if let Some(v) = calculate_v(
                    signature
                        .r
                        .into_inner()
                        .to_be_bytes()
                        .as_bytes()
                        .try_into()
                        .ok()
                        .unwrap(),
                    signature
                        .s
                        .into_inner()
                        .to_be_bytes()
                        .as_bytes()
                        .try_into()
                        .ok()
                        .unwrap(),
                    data.clone(),
                    key_share
                        .shared_public_key
                        .to_bytes(true)
                        .as_bytes()
                        .try_into()
                        .ok()
                        .unwrap(),
                ) {
                    v
                } else {
                    tracing::error!("failed to calculate v");
                    handle.abort();
                    return;
                };

                println!("v {:?}", v);

                let mut formatted_signature = Vec::new();
                formatted_signature
                    .extend_from_slice(&signature.r.into_inner().to_be_bytes().as_bytes());
                formatted_signature
                    .extend_from_slice(&signature.s.into_inner().to_be_bytes().as_bytes());
                formatted_signature.extend_from_slice(&[v]);

                let serialized_formatted_signature = hex::encode(formatted_signature);
                tracing::info!(
                    "succesfully signed with signer_id={local_signer_id} and eid={eid_clone_for_signing}. data={}: signature={:?} ",
                    hex::encode(&data),
                    serialized_formatted_signature
                );

                response_tx.send(serialized_formatted_signature).unwrap();
            }
            Err(err) => {
                // TODO: send back the error
                tracing::error!("error during signature generation. reason: {err}");
                handle.abort();
            }
        }
    });

    return Ok(tx_0);
}

pub fn calculate_v(
    r: [u8; 32],
    s: [u8; 32],
    data: Vec<u8>,
    shared_public_key: [u8; 33], // Adjusted to match secp256k1 compressed format
) -> Option<u8> {
    let message_hash = Sha256::digest(&data);
    let message_hash: [u8; 32] = message_hash.try_into().unwrap();

    let signature = Signature::from_scalars(r, s).ok()?;

    for recovery_id_byte in 0..4 {
        let recovery_id = RecoveryId::from_byte(recovery_id_byte).unwrap();

        if let Ok(verifying_key) =
            VerifyingKey::recover_from_prehash(&message_hash, &signature, recovery_id)
        {
            let public_key_bytes = verifying_key.to_encoded_point(true);
            if public_key_bytes.as_bytes() == shared_public_key {
                return Some(recovery_id_byte + 27);
            }
        }
    }

    None
}
