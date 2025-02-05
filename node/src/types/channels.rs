use futures::{self, channel::oneshot};
use serde::{Deserialize, Serialize};

use crate::mpc::mpc_service::MpcServiceError;

#[derive(Serialize, Deserialize)]
pub enum EventCommands {
    NewEvent { data: Vec<u8> },
}

pub enum RpcCommands {
    StartGenerateKey {
        response_tx: oneshot::Sender<Result<String, MpcServiceError>>,
    },
    StartSigning {
        response_tx: oneshot::Sender<Result<String, MpcServiceError>>,
        shared_public_key: String, // hex encoded
        data: Vec<u8>,
    },
}

#[derive(Serialize, Deserialize)]
pub enum ProtocolEvents {
    JoinAuxInfoGen {
        eid: String,
    },
    JoinKeyGen {
        eid: String,
    },
    JoinSigning {
        eid: String,
        shared_public_key: String,
        data: Vec<u8>,
    },
    Cggmp21CoreAuxInfoGen {
        eid: String,
        message_id: u64,
        data: Vec<u8>,
        signer_id: u16,
        receiver_signer_id: Option<u16>,
    },
    Cggmp21CoreKeygen {
        eid: String,
        message_id: u64,
        data: Vec<u8>,
        signer_id: u16,
        receiver_signer_id: Option<u16>,
    },
    Cggmp21CoreSigning {
        eid: String,
        message_id: u64,
        data: Vec<u8>,
        signer_id: u16,
        receiver_signer_id: Option<u16>,
    },
}

pub enum NetworkCommands {
    Send { data: Vec<u8> },
    GetLocalSignerId { response_tx: oneshot::Sender<u16> },
    AllPeersJoined,
}
