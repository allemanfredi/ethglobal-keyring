use anyhow::Result;
use libp2p::PeerId;

#[derive(Clone)]
pub struct PeersManager {
    peers: Vec<PeerId>,
}

impl PeersManager {
    pub fn new() -> Self {
        PeersManager { peers: vec![] }
    }

    pub fn add_peer(&mut self, peer_id: PeerId) -> Result<()> {
        self.peers.push(peer_id);
        Ok(())
    }

    pub fn signer_id_of(&self, peer_id: PeerId) -> Result<u16, ()> {
        let mut sorted = self.peers.clone();
        sorted.sort();

        match sorted.binary_search(&peer_id) {
            Ok(index) => Ok(index as u16),
            Err(_) => Err(()),
        }
    }

    pub fn get_total_number_of_peers(&self) -> u64 {
        self.peers.len() as u64
    }

    pub fn peers(&self) -> Vec<PeerId> {
        self.peers.clone()
    }

    pub fn is_connected(&self, peer_id: PeerId) -> bool {
        self.peers.contains(&peer_id)
    }
}
