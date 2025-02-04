#[macro_use]
extern crate lazy_static;

mod p2p;
mod rpc;
mod types;
mod utils;

use anyhow::Result;
use futures::{self, channel::mpsc::unbounded};
use p2p::network_service::NetworkService;
use rand_core::OsRng;
use rpc::rpc_service::RpcService;
use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};
use tokio::{self};
use tracing_subscriber::{self, util::SubscriberInitExt};
use types::channels::{EventCommands, NetworkCommands, RpcCommands};

#[tokio::main]
async fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?;
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .finish()
        .try_init()?;

    let rpc_service_listen_addr = std::env::args().nth(1).unwrap();
    let network_service_listen_addr = std::env::args().nth(2).unwrap();
    let remote_peers = std::env::args()
        .nth(3)
        .unwrap()
        .split(',')
        .map(String::from)
        .collect();

    let rpc_service = RpcService::new(rpc_service_listen_addr);
    let mut network_service = NetworkService::new(network_service_listen_addr, remote_peers);

    tokio::spawn(async move {
        rpc_service.start().await.unwrap();
    });

    tokio::spawn(async move {
        network_service.start().await;
    });

    futures::future::pending().await
}
