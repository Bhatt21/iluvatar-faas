// iluvatar_lib/src/http/mod.rs

pub mod http_server;
use anyhow::Result;

use std::net::SocketAddr;

/// Creates an HttpServer instance from a given address and port.
pub async fn create_http_server(addr: &str, port: u16) ->Result< http_server::HttpServer > {
    let socket_addr: SocketAddr = format!("{}:{}", addr, port)
        .parse()
        .expect("Invalid address or port provided");
    Ok(http_server::HttpServer::new(socket_addr))
}
