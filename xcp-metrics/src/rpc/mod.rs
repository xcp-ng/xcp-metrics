//! RPC routes and entrypoint.
pub mod routes;
pub mod daemon;

use std::{collections::HashMap, sync::Arc};

use xcp_metrics_common::{
    rpc::message::{RpcError, RpcRequest},
    xapi::hyper::{Body, Request, Response},
};

use crate::{rpc::routes::generate_routes, XcpMetricsShared};

#[tracing::instrument(skip_all)]
pub async fn route(
    shared: Arc<XcpMetricsShared>,
    request: RpcRequest,
    rpc_routes: &HashMap<&str, Box<dyn routes::XcpRpcRoute>>,
) -> anyhow::Result<Response<Body>> {
    tracing::info!("RPC: Message: {request}");

    if let Some(route) = rpc_routes.get(request.get_name()) {
        route.run(shared, request).await
    } else {
        tracing::error!("RPC: Method not found: {request}");
        RpcError::respond_to::<()>(Some(&request), -32601, "Method not found", None)
    }
}

#[tracing::instrument(skip_all)]
pub async fn entrypoint(
    shared: Arc<XcpMetricsShared>,
    request: Request<Body>,
) -> anyhow::Result<Response<Body>> {
    let rpc_routes = generate_routes();
    tracing::debug!("RPC: {request:#?}");

    let request = RpcRequest::from_http(request).await;

    match request {
        Ok(request) => route(shared, request, &rpc_routes).await,
        Err(err) => {
            tracing::error!("RPC: Parse error: {err}");
            RpcError::respond_to(None, -32700, "Parse error", Some(err.to_string()))
        }
    }
}
