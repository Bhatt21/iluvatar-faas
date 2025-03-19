// iluvatar_lib/src/http/http_server.rs

use axum::{Router, routing::get};
use std::net::SocketAddr;
use anyhow::Result;
use tokio::net::TcpListener;
use tracing::info;

#[derive(Clone)]
pub struct HttpServer {
    pub addr: SocketAddr,
}

impl HttpServer {
    /// Creates a new HttpServer instance with the given address.
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    #[tracing::instrument(skip(self))]
    pub async fn run(self) -> Result<()> {
        let app = Router::new().route("/ping", get(Self::handle_ping));
        info!(address=%self.addr, "Starting HTTP listener");
        let listener = TcpListener::bind(self.addr).await?;
        info!("serving now");
        axum::serve(listener, app.into_make_service()).await?;
        Ok(())
        
    }

    async fn handle_ping() -> &'static str {
        "pong"
    }
}
