#[macro_use]
extern crate lazy_static;

mod rpc;
mod types;
mod utils;

use std::{
    fs::File,
    io::{Read, Write},
    path::Path,
};

use anyhow::Result;
use futures::{self, channel::mpsc::unbounded};
use rand_core::OsRng;
use rpc::RpcService;
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

    let rpc_service = RpcService::new(rpc_service_listen_addr);

    tokio::spawn(async move {
        rpc_service.start().await.unwrap();
    });

    futures::future::pending().await
}
