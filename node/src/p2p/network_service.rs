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
}

impl NetworkService {
    pub fn new(listen_addr: String, peers_multi_addr: Vec<String>) -> Self {
        NetworkService {
            listen_addr,
            peers_multi_addr,
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
                              // TODO:

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
                                    // TODO: all peers joined
                                }
                            } else {
                                tracing::info!("{peer_id} already connected. skipping it ...");
                            }

                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
