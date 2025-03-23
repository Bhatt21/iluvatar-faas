
pub mod http_server;
pub mod handlers;
use anyhow::Result;

use std::net::SocketAddr;

/// Creates an HttpServer instance from a given address and port.
pub async fn create_http_server(addr: &str, port: u16, rpc_host: &str, rpc_port: u16) ->Result< http_server::HttpServer > {
    let socket_addr: SocketAddr = format!("{}:{}", addr, port)
        .parse()
        .expect("Invalid address or port provided");
    Ok(http_server::HttpServer::new(socket_addr, rpc_host.to_string(), rpc_port))
}
