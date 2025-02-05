use anyhow::Result;
use cggmp21::{
    key_share::{DirtyAuxInfo, Valid},
    keygen::msg::threshold::Msg,
    progress::PerfProfiler,
    round_based::{Incoming, MessageDestination, MpcParty, Outgoing},
    security_level::SecurityLevel128,
    supported_curves::Secp256k1,
    KeygenError as CGGMP21KeygenError,
};
use futures::{
    channel::{
        mpsc::{self, UnboundedSender},
        oneshot,
    },
    StreamExt,
};
use k256::sha2::Sha256;
use rand::Rng;
use rand_core::OsRng;

use crate::types::channels::{NetworkCommands, ProtocolEvents};

#[derive(Debug, thiserror::Error)]
pub enum KeygenError {}

pub async fn run_keygen(
    network_channel_tx: mpsc::UnboundedSender<NetworkCommands>,
    response_tx: oneshot::Sender<String>, // oneshot::Sender<Valid<DirtyKeyShare<Secp256k1, SecurityLevel128>>> doesn't implement Debug ,
    send_join_keygen_event: bool,
    eid: String,
    local_signer_id: u16,
    aux_info: Valid<DirtyAuxInfo>,
    number_of_participants: u16,
    threshold: u16,
) -> Result<UnboundedSender<Result<Incoming<Msg<Secp256k1, SecurityLevel128, Sha256>>, KeygenError>>>
{
    let (tx_0, rx_0) = mpsc::unbounded::<
        Result<Incoming<Msg<Secp256k1, SecurityLevel128, Sha256>>, KeygenError>,
    >();
    let (tx_1, mut rx_1) = mpsc::unbounded::<Outgoing<Msg<Secp256k1, SecurityLevel128, Sha256>>>();
    let party = MpcParty::connected((rx_0, tx_1));

    if send_join_keygen_event {
        // NOTE: trigger node-keyring to call .set_threshold().start()
        let event = ProtocolEvents::JoinKeyGen { eid: eid.clone() };
        let event_bytes = bincode::serialize(&event).unwrap();
        network_channel_tx
            .unbounded_send(NetworkCommands::Send { data: event_bytes })
            .unwrap();
    }

    // NOTE: forward messages to swarm
    let eid_clone_for_keygen = eid.clone();
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                outgoing = rx_1.next() => {
                    match outgoing {
                        Some(outgoing) => {
                            let event = ProtocolEvents::Cggmp21CoreKeygen {
                                eid:eid.clone(),
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
        let mut tracer = PerfProfiler::new();
        let signer_id: u16 = local_signer_id;
        match cggmp21::keygen::KeygenBuilder::new(
            cggmp21::ExecutionId::new(eid_clone_for_keygen.as_bytes()),
            signer_id,
            number_of_participants,
        )
        .set_progress_tracer(&mut tracer)
        .set_threshold(threshold)
        .start(&mut OsRng, party)
        .await
        {
            Ok(incomplete_key_share) => {
                let key_share =
                    cggmp21::KeyShare::from_parts((incomplete_key_share, aux_info)).unwrap();

                tracing::info!(
                    "succesfully generate the keyshare with signer_id={signer_id} for eid={eid_clone_for_keygen}"
                );

                let serialized_key_share = serde_json::to_string(&key_share).unwrap();
                tracing::debug!("{:?}", serialized_key_share);
                response_tx.send(serialized_key_share).unwrap();
            }
            Err(err) => {
                // TODO: send back the error
                tracing::error!("error during key generation. reason: {err}");
                handle.abort();
            }
        }
    });

    return Ok(tx_0);
}
