use anyhow::Result;
use futures::{
    channel::mpsc::{self},
    SinkExt, StreamExt,
};
use libp2p::{
    noise,
    request_response::{self, ProtocolSupport},
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, StreamProtocol,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::types::channels::{EventCommands, NetworkCommands};

use super::peers_manager::PeersManager;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Request(Vec<u8>);
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Response(Vec<u8>);

#[derive(NetworkBehaviour)]
struct KeyringBehavior {
    request_response: request_response::cbor::Behaviour<Request, Response>,
}

pub struct NetworkService {
    listen_addr: String,
    peers_multi_addr: Vec<String>,
    network_channel_tx: mpsc::UnboundedSender<NetworkCommands>,
    network_channel_rx: mpsc::UnboundedReceiver<NetworkCommands>,
    protocol_event_channel_tx: mpsc::UnboundedSender<EventCommands>,
}

impl NetworkService {
    pub fn new(
        listen_addr: String,
        peers_multi_addr: Vec<String>,
        network_channel_tx: mpsc::UnboundedSender<NetworkCommands>,
        network_channel_rx: mpsc::UnboundedReceiver<NetworkCommands>,
        protocol_event_channel_tx: mpsc::UnboundedSender<EventCommands>,
    ) -> Self {
        NetworkService {
            listen_addr,
            peers_multi_addr,
            network_channel_tx,
            network_channel_rx,
            protocol_event_channel_tx,
        }
    }

    pub async fn start(&mut self) {
        let mut swarm = libp2p::SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .unwrap()
            .with_behaviour(|_| KeyringBehavior {
                request_response: request_response::cbor::Behaviour::new(
                    [(StreamProtocol::new("/keyring/1"), ProtocolSupport::Full)],
                    request_response::Config::default(),
                ),
            })
            .unwrap()
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        swarm.listen_on(self.listen_addr.parse().unwrap()).unwrap();

        for peer_multi_addr in self.peers_multi_addr.iter().into_iter() {
            let remote: Multiaddr = peer_multi_addr.parse().unwrap();
            swarm.dial(remote).unwrap();
        }

        let peers_manager = PeersManager::new();
        let mut peers_manager = peers_manager;
        peers_manager.add_peer(*swarm.local_peer_id()).unwrap();

        loop {
            tokio::select! {
                // Handle swarm events
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::Behaviour(KeyringBehaviorEvent::RequestResponse(
                            request_response::Event::Message { message, .. },
                        )) => match message {
                            request_response::Message::Request {
                                request, channel, ..
                            } => {
                               self.protocol_event_channel_tx.unbounded_send(EventCommands::NewEvent { data: request.0 }).unwrap();
                               self.protocol_event_channel_tx.flush().await.unwrap();
                               // NOTE: send an empty response
                               //swarm.behaviour_mut().request_response.send_response(channel, Response(vec![])).unwrap();

                            }
                            request_response::Message::Response {
                                request_id,
                                response,
                            } => {

                            }
                        },
                        SwarmEvent::ConnectionEstablished {
                            peer_id, ..
                        } => {
                            if !peers_manager.is_connected(peer_id) {
                                tracing::info!("connection estabilished with {peer_id}");
                                peers_manager
                                    .add_peer(peer_id)
                                    .unwrap();

                                // NOTE: When all peers join the network during the bootstrap phase
                                // we have to notify the mpc service which will start the auxiliary info generation
                                if peers_manager.get_total_number_of_peers() == 3 {
                                    self.network_channel_tx.unbounded_send(NetworkCommands::AllPeersJoined).unwrap();
                                }
                            } else {
                                tracing::info!("{peer_id} already connected. skipping it ...");
                            }

                        }
                        _ => {}
                    }
                },
                // Handle messages from rpc service
                cmd = self.network_channel_rx.next() => {
                    if let Some(cmd) = cmd {
                        match cmd {
                            NetworkCommands::Send { data } => {
                                // TODO: send only to specific sender
                                let peers_ids = peers_manager.peers();
                                for peer_id in peers_ids {
                                    if peer_id != *swarm.local_peer_id() {
                                        swarm.behaviour_mut().request_response.send_request(&peer_id, Request(data.clone()));
                                    }
                                }
                            }
                            NetworkCommands::GetLocalSignerId { response_tx } => {
                                // TODO: check that all peers are connected or that at least the node knows them
                                // in order to assign properly the rank which will be used as signer id
                                let signer_id = peers_manager
                                    .signer_id_of(*swarm.local_peer_id()).unwrap();

                                response_tx.send(signer_id).unwrap();
                            },
                            _ => {
                                tracing::info!("received an invalid network command");
                            }
                        }
                    }
                }
            }
        }
    }
}
