// iluvatar_lib/src/http/http_server.rs
use axum::{
    extract::{Extension, Path, Query, Json},
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use anyhow::Result;
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use std::collections::HashMap;
use tracing::{info,debug};
use iluvatar_worker_library::worker_api::{rpc::RPCWorkerAPI, WorkerAPI};
use iluvatar_library::transaction::gen_tid;
use serde_json::json;
use iluvatar_library::types::{Compute, ContainerServer, Isolation};

#[derive(Clone)]
pub struct HttpServer {
    pub addr: SocketAddr,
    pub rpc_host: String,
    pub rpc_port: u16,
}

#[derive(Debug, Deserialize)]
struct RegisterParams {
    function_name: String,
    image: String,
    version: String,
    memory: u32,
    cpu: u32,
    isolate: String,
    compute: String,
}

impl HttpServer {
    /// Creates a new HttpServer instance with the given address, rpc_host, and rpc_port.
    pub fn new(addr: SocketAddr, rpc_host: String, rpc_port: u16) -> Self {
        Self { addr, rpc_host, rpc_port }
    }

    /// Helper function to create the RPC client.
    async fn create_rpc_client(&self, tid: &str) -> Result<RPCWorkerAPI, String> {
        let tid_string = tid.to_string();
        RPCWorkerAPI::new(&self.rpc_host, self.rpc_port, &tid_string)
            .await
            .map_err(|e| format!("Error initializing RPC client: {:?}", e))
    }

    #[tracing::instrument(skip(self))]
    pub async fn run(self) -> Result<()> {
        // Build the Axum router and share the server state via an extension.
        let app = Router::new()
            .route("/ping", get(Self::handle_ping))
            .route("/register", post(Self::handle_register))
            .route("/invoke/:func_name/:version", get(Self::handle_invoke))
            .route("/async_invoke/:func_name/:version", get(Self::handle_async_invoke))
            .route("/invoke_async_check/:cookie", get(Self::handle_async_invoke_check))
            .route("/list_functions", get(Self::handle_list_registered_funcs))
            .layer(Extension(self.clone()));

        info!(address=%self.addr, "Starting HTTP listener");
        let listener = TcpListener::bind(self.addr).await?;
        axum::serve(listener, app.into_make_service()).await?;
        Ok(())
    }

    async fn handle_ping(Extension(server): Extension<HttpServer>) -> impl IntoResponse {
        let tid = gen_tid();
        let mut api = match server.create_rpc_client(&tid).await {
            Ok(api) => api,
            Err(e) => return e,
        };
        let ret = api.ping(tid).await.unwrap();
        debug!("{}", ret);
        ret
    }

    async fn handle_register(
        Extension(server): Extension<HttpServer>,
        Json(params): Json<RegisterParams>
    ) -> impl IntoResponse {
        // Validate isolation.
        let isolation_str = params.isolate.to_lowercase();
        let isolation = match isolation_str.as_str() {
            "docker" => vec![Isolation::DOCKER],
            "containerd" => vec![Isolation::CONTAINERD],
            _ => {
                return format!("Error: isolation must be either 'docker' or 'containerd'");
            }
        };

        // Validate compute.
        let compute_str = params.compute.to_uppercase();
        let compute = match compute_str.as_str() {
            "CPU" => vec![Compute::CPU],
            "GPU" => vec![Compute::GPU],
            _ => {
                return format!("Error: compute must be either 'CPU' or 'GPU'");
            }
        };

        let tid = gen_tid();
        let image = params.image;
        let mem_size_mb: i64 = params.memory as i64; // casting memory to i64
        let server_type = ContainerServer::HTTP;

        // Create RPC client using the helper method.
        let mut api = match server.create_rpc_client(&tid).await {
            Ok(api) => api,
            Err(e) => return e,
        };

        // Call register.
        let ret = match api
            .register(
                params.function_name,
                params.version,
                image,
                mem_size_mb,
                params.cpu,
                1, // some_constant
                tid,
                isolation.into(),
                compute.into(),
                server_type,
                None,
            )
            .await
        {
            Ok(result) => result,
            Err(e) => return format!("Error during RPC register call: {:?}", e),
        };

        info!("Register RPC returned: {}", ret);
        ret
    }

    async fn handle_invoke(
        Extension(server): Extension<HttpServer>,
        Path((func_name, version)): Path<(String, String)>,
        Query(query_params): Query<HashMap<String, String>>,
    ) -> impl IntoResponse {
        let arguments = match serde_json::to_string(&query_params) {
            Ok(json_str) => json_str,
            Err(e) => return format!("Error converting query parameters to JSON: {:?}", e),
        };

        let tid = gen_tid();
        let mut api = match server.create_rpc_client(&tid).await {
            Ok(api) => api,
            Err(e) => return e,
        };

        let ret = match api.invoke(func_name, version, arguments, tid).await {
            Ok(result) => result,
            Err(e) => return format!("Error during RPC invoke call: {:?}", e),
        };

        info!("RPC invoke returned: {:?}", ret);
        serde_json::to_string(&ret)
            .unwrap_or_else(|e| format!("Error converting result to JSON: {:?}", e))
    }

    async fn handle_async_invoke(
        Extension(server): Extension<HttpServer>,
        Path((func_name, version)): Path<(String, String)>,
        Query(query_params): Query<HashMap<String, String>>,
    ) -> impl IntoResponse {
        let arguments = match serde_json::to_string(&query_params) {
            Ok(json_str) => json_str,
            Err(e) => return format!("Error converting query parameters to JSON: {:?}", e),
        };

        let tid = gen_tid();
        let mut api = match server.create_rpc_client(&tid).await {
            Ok(api) => api,
            Err(e) => return e,
        };

        let ret = match api.invoke_async(func_name, version, arguments, tid).await {
            Ok(cookie) => cookie,
            Err(e) => return format!("Error during RPC invoke_async call: {:?}", e),
        };

        info!("RPC async_invoke returned: {}", ret);
        ret
    }

    async fn handle_async_invoke_check(
        Extension(server): Extension<HttpServer>,
        Path(cookie): Path<String>,
    ) -> impl IntoResponse {
        let tid = gen_tid();
        let mut api = match server.create_rpc_client(&tid).await {
            Ok(api) => api,
            Err(e) => return e,
        };

        let ret = match api.invoke_async_check(&cookie, gen_tid()).await {
            Ok(result) => result,
            Err(e) => return format!("Error during RPC invoke_async_check call: {:?}", e),
        };

        serde_json::to_string(&ret)
            .unwrap_or_else(|e| format!("Error converting result to JSON: {:?}", e))
    }

    async fn handle_list_registered_funcs(
        Extension(server): Extension<HttpServer>
    ) -> impl IntoResponse {
        let tid = gen_tid();
        let mut api = match server.create_rpc_client(&tid).await {
            Ok(api) => api,
            Err(e) => return e,
        };

        let ret = match api.list_registered_funcs(gen_tid()).await {
            Ok(result) => result,
            Err(e) => return format!("Error during RPC call: {:?}", e),
        };

        let functions = ret
            .functions
            .into_iter()
            .map(|func| {
                json!({
                    "function_name": func.function_name,
                    "function_version": func.function_version,
                    "image_name": func.image_name,
                })
            })
            .collect::<Vec<_>>();

        let output = json!({ "functions": functions });
        serde_json::to_string(&output)
            .unwrap_or_else(|e| format!("Error converting result to JSON: {:?}", e))
    }
}
