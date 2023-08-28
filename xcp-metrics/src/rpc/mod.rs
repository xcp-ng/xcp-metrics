//! RPC routes and entrypoint.
pub mod daemon;
pub mod routes;

use std::sync::Arc;

use xapi::{
    hyper::{Body, Request, Response},
    rpc::message::{RpcError, RpcRequest},
};

use crate::XcpMetricsShared;

#[tracing::instrument(skip_all)]
pub async fn route(
    shared: Arc<XcpMetricsShared>,
    request: RpcRequest,
) -> anyhow::Result<Response<Body>> {
    tracing::info!("RPC: Message: {request}");

    if let Some(route) = shared.clone().rpc_routes.get(request.get_name()) {
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
    tracing::debug!("RPC: {request:#?}");

    let request = RpcRequest::from_http(request).await;

    match request {
        Ok(request) => route(shared, request).await,
        Err(err) => {
            tracing::error!("RPC: Parse error: {err}");
            RpcError::respond_to(None, -32700, "Parse error", Some(err.to_string()))
        }
    }
}
