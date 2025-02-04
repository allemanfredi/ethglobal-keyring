use futures::channel::mpsc;
use hyper::Method;
use jsonrpsee::server::Server;
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

use crate::{
    rpc::methods::{Methods, RpcServer},
    RpcCommands,
};

pub struct RpcService {
    listen_addr: String,
}

impl RpcService {
    pub fn new(listen_addr: String) -> Self {
        RpcService {
            listen_addr,
        }
    }

    pub async fn start(&self) -> anyhow::Result<SocketAddr> {
        let cors = CorsLayer::new()
            .allow_methods([Method::POST])
            .allow_origin(Any)
            .allow_headers([hyper::header::CONTENT_TYPE]);
        let middleware = tower::ServiceBuilder::new().layer(cors);

        let server = Server::builder()
            //.set_http_middleware(middleware)
            .build(self.listen_addr.parse::<SocketAddr>()?)
            .await?;

        let addr = server.local_addr()?;
        let methods = Methods {};
        let handle = server.start(methods.into_rpc());

        tokio::spawn(handle.stopped());

        tracing::info!("rpc server started on {addr}");

        Ok(addr)
    }
}
