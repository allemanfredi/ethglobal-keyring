use anyhow::Result;
use cggmp21::{
    key_refresh::AuxOnlyMsg as Msg,
    key_share::{DirtyAuxInfo, Valid},
    progress::PerfProfiler,
    round_based::{Incoming, MessageDestination, MpcParty, Outgoing},
    security_level::SecurityLevel128,
    PregeneratedPrimes,
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
pub enum AuxInfoGenErrors {}

pub async fn run_aux_info_gen(
    network_channel_tx: mpsc::UnboundedSender<NetworkCommands>,
    response_tx: oneshot::Sender<String>, // Valid<DirtyAuxInfo> doesn't implement Debug
    send_join_aux_info_gen_event: bool,
    local_signer_id: u16,
    eid: String,
    pregenerated_primes: PregeneratedPrimes,
) -> Result<UnboundedSender<Result<Incoming<Msg<Sha256, SecurityLevel128>>, AuxInfoGenErrors>>> {
    let (tx_0, rx_0) =
        mpsc::unbounded::<Result<Incoming<Msg<Sha256, SecurityLevel128>>, AuxInfoGenErrors>>();
    let (tx_1, mut rx_1) = mpsc::unbounded::<Outgoing<Msg<Sha256, SecurityLevel128>>>();
    let party = MpcParty::connected((rx_0, tx_1));

    if send_join_aux_info_gen_event {
        // NOTE: trigger node-keyring to call .aux_info_gen()
        let event = ProtocolEvents::JoinAuxInfoGen { eid: eid.clone() };
        let event_bytes = bincode::serialize(&event).unwrap();
        network_channel_tx
            .unbounded_send(NetworkCommands::Send { data: event_bytes })
            .unwrap();
    }

    // NOTE: forward messages to swarm
    let eid_clone_for_aux_info_gen = eid.clone();
    let handle = tokio::spawn(async move {
        loop {
            tokio::select! {
                outgoing = rx_1.next() => {
                    match outgoing {
                        Some(outgoing) => {
                            let event = ProtocolEvents::Cggmp21CoreAuxInfoGen {
                                eid: eid.clone(),
                                message_id: OsRng.gen::<u64>(),
                                data: bincode::serialize(&outgoing.msg).unwrap(),
                                signer_id: local_signer_id,
                                receiver_signer_id: match outgoing.recipient {
                                    MessageDestination::OneParty(index) => Some(index),
                                    _ => None,
                                }
                            };
                            let event_bytes = bincode::serialize(&event).unwrap();
                            network_channel_tx.unbounded_send(NetworkCommands::Send { data: event_bytes }).unwrap();
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
        let signer_id = local_signer_id;
        let number_of_participants = 3;

        match cggmp21::aux_info_gen(
            cggmp21::ExecutionId::new(eid_clone_for_aux_info_gen.as_bytes()),
            signer_id,
            number_of_participants,
            pregenerated_primes,
        )
        .set_progress_tracer(&mut tracer)
        .start(&mut OsRng, party)
        .await
        {
            Ok(aux_info) => {
                tracing::info!("succesfully generated the auxiliary info with signer_id={signer_id} for eid={eid_clone_for_aux_info_gen}");
                response_tx
                    .send(serde_json::to_string(&aux_info).unwrap())
                    .unwrap();
            }
            Err(err) => {
                // TODO: send back the error
                tracing::error!("failed to generate the auxiliary info. reason: {err}");
                handle.abort();
            }
        }
    });

    return Ok(tx_0);
}
