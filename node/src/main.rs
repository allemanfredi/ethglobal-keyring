#[macro_use]
extern crate lazy_static;

mod eigen_layer;
mod mpc;
mod p2p;
mod rpc;
mod types;
mod utils;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use anyhow::Result;
use cggmp21::PregeneratedPrimes;
// use eigen_layer::{EigenLayerService, EigenLayerServiceConfig};
use futures::{self, channel::mpsc::unbounded};
use mpc::mpc_service::MpcService;
use p2p::network_service::NetworkService;
use rand_core::OsRng;
use rpc::rpc_service::RpcService;
use tokio::{self};
use tracing_subscriber::{self, util::SubscriberInitExt};
use types::channels::{EventCommands, NetworkCommands, RpcCommands};

fn load_or_generate_primes(
    pregenerated_primes_filename: &str,
) -> Result<PregeneratedPrimes, Box<dyn std::error::Error>> {
    if Path::new(&pregenerated_primes_filename).exists() {
        tracing::info!("loading primes from {pregenerated_primes_filename} ...");
        let mut file = File::open(pregenerated_primes_filename)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let pregenerated_primes: PregeneratedPrimes = serde_json::from_str(&contents).unwrap();
        Ok(pregenerated_primes)
    } else {
        tracing::info!("generating primes before starting ...");
        let pregenerated_primes = cggmp21::PregeneratedPrimes::generate(&mut OsRng);
        tracing::info!("primes succesfully generated");
        let serialized_pregenerated_primes =
            serde_json::to_string_pretty(&pregenerated_primes).unwrap();
        let mut file = File::create(&pregenerated_primes_filename).unwrap();
        file.write_all(serialized_pregenerated_primes.as_bytes())?;
        Ok(pregenerated_primes)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?;
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .finish()
        .try_init()?;

    let remote_peers = std::env::args()
        .nth(1)
        .unwrap()
        .split(',')
        .map(String::from)
        .collect();
    let network_service_listen_addr = std::env::args().nth(2).unwrap();
    let rpc_service_listen_addr = std::env::args().nth(3).unwrap();
    let pregenerated_primes_filename = std::env::args().nth(4).unwrap();

    let pregenerated_primes = load_or_generate_primes(&pregenerated_primes_filename).unwrap();

    /*let eigen_layer_service = EigenLayerService::new(EigenLayerServiceConfig {
        bls_secret_key:
            "12248929636257230549931416853095037629726205319386239410403476017439825112537"
                .to_string(),
        private_key: "835c5faf238c2731c6279a2953761e33fa44817689df2ab7ca27bf824b9d39ba".to_string(), // 0xe48f51fbA0f05Aa79c2E332c610F0e05e14CE004
        provider: "https://ethereum-holesky-rpc.publicnode.com".to_string(),
    });*/

    // NOTE: create a channel between the RpcService and the NetworkService
    let (rpc_channel_tx, rpc_channel_rx) = unbounded::<RpcCommands>();
    let (network_channel_tx_0, network_channel_rx_0) = unbounded::<NetworkCommands>();
    let (network_channel_tx_1, network_channel_rx_1) = unbounded::<NetworkCommands>();
    let (protocol_event_channel_tx, protocol_event_channel_rx) = unbounded::<EventCommands>();

    let rpc_service = RpcService::new(rpc_service_listen_addr, rpc_channel_tx);
    let mut mpc_service = MpcService::new(
        rpc_channel_rx,
        network_channel_tx_0,
        network_channel_rx_1,
        protocol_event_channel_rx,
        pregenerated_primes,
    );

    let mut network_service = NetworkService::new(
        network_service_listen_addr,
        remote_peers,
        network_channel_tx_1,
        network_channel_rx_0,
        protocol_event_channel_tx,
    );

    tokio::spawn(async move {
        rpc_service.start().await.unwrap();
    });

    tokio::spawn(async move {
        network_service.start().await;
    });

    tokio::spawn(async move {
        mpc_service.start().await;
    });

    futures::future::pending().await
}
