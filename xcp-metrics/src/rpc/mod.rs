//! RPC routes and entrypoint.
pub mod routes;
pub mod xapi;

use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;
use tokio::{sync::mpsc, task::JoinHandle};
use xcp_metrics_common::{
    rpc::message::{RpcError, RpcRequest},
    xapi::hyper::{Body, Request, Response},
};

use crate::{hub::HubPushMessage, rpc::routes::generate_routes};

#[derive(Default, Debug)]
pub struct RpcShared {
    pub plugins: DashMap<Box<str>, JoinHandle<()>>,
}

#[tracing::instrument(skip_all)]
pub async fn route(
    shared: Arc<RpcShared>,
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    request: RpcRequest,
    rpc_routes: &HashMap<&str, Box<dyn routes::XcpRpcRoute>>,
) -> anyhow::Result<Response<Body>> {
    tracing::info!("RPC: Message: {request}");

    if let Some(route) = rpc_routes.get(request.get_name()) {
        route.run(shared, hub_channel, request).await
    } else {
        RpcError::respond_to::<()>(Some(&request), -32601, "Method not found", None)
    }
}

#[tracing::instrument(skip_all)]
pub async fn entrypoint(
    shared: Arc<RpcShared>,
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    request: Request<Body>,
) -> anyhow::Result<Response<Body>> {
    let rpc_routes = generate_routes();
    tracing::debug!("RPC: {request:#?}");

    let request = RpcRequest::from_http(request).await;

    match request {
        Ok(request) => route(shared, hub_channel, request, &rpc_routes).await,
        Err(err) => RpcError::respond_to(None, -32700, "Parse error", Some(err.to_string())),
    }
}
