use axum::{
    extract::{Extension, Path, Query, Json},
    response::IntoResponse,
};
use serde_json::json;
use std::collections::HashMap;
use tracing::info;
use iluvatar_worker_library::worker_api::{rpc::RPCWorkerAPI, WorkerAPI};
use iluvatar_library::transaction::gen_tid;
use iluvatar_library::types::{Compute, ContainerServer, Isolation};

use crate::HttpServer;

/// Handler for the /ping route.
pub async fn handle_ping(Extension(server): Extension<HttpServer>) -> String {
    let tid = gen_tid();
    let mut api = match server.create_rpc_client(&tid).await {
        Ok(api) => api,
        Err(e) => return e,
    };
    let ret = api.ping(tid).await.unwrap();
    info!("{}", ret);
    ret
}

/// Handler for the /register route.
pub async fn handle_register(
    Extension(server): Extension<HttpServer>,
    Json(params): Json<crate::RegisterParams>,
) -> impl IntoResponse {
    // Validate isolation.
    let isolation_str = params.isolate.to_lowercase();
    let isolation = match isolation_str.as_str() {
        "docker" => vec![Isolation::DOCKER],
        "containerd" => vec![Isolation::CONTAINERD],
        _ => return format!("Error: isolation must be either 'docker' or 'containerd'"),
    };

    // Validate compute.
    let compute_str = params.compute.to_uppercase();
    let compute = match compute_str.as_str() {
        "CPU" => vec![Compute::CPU],
        "GPU" => vec![Compute::GPU],
        _ => return format!("Error: compute must be either 'CPU' or 'GPU'"),
    };

    let tid = gen_tid();
    let image = params.image;
    let mem_size_mb: i64 = params.memory as i64; // casting memory to i64
    let server_type = ContainerServer::HTTP;

    let mut api = match server.create_rpc_client(&tid).await {
        Ok(api) => api,
        Err(e) => return e,
    };

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

/// Handler for the /invoke/:func_name/:version route.
pub async fn handle_invoke(
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

/// Handler for the /async_invoke/:func_name/:version route.
pub async fn handle_async_invoke(
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

/// Handler for the /invoke_async_check/:cookie route.
pub async fn handle_async_invoke_check(
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

/// Handler for the /list_registered_func route.
pub async fn handle_list_registered_funcs(
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
