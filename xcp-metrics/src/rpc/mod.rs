pub mod routes;
pub mod xapi;

use std::{collections::HashMap, sync::Arc};

use anyhow::bail;
use dashmap::DashSet;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    rpc::message::RpcRequest,
    xapi::hyper::{Body, Request, Response},
};

use crate::{hub::HubPushMessage, rpc::routes::generate_routes};

#[derive(Default)]
pub struct RpcShared {
    plugins: DashSet<Box<str>>,
}

pub async fn route(
    shared: Arc<RpcShared>,
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    request: RpcRequest,
    rpc_routes: &HashMap<&str, Box<dyn routes::XcpRpcRoute>>,
) -> anyhow::Result<Response<Body>> {
    println!("RPC: {:?}", &request);

    if let Some(route) = rpc_routes.get(request.get_name()) {
        route.run(shared, hub_channel, request).await
    } else {
        Ok(Response::builder().body("Unknown RPC method".into())?)
    }
}

pub async fn entrypoint(
    shared: Arc<RpcShared>,
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    req: Request<Body>,
) -> anyhow::Result<Response<Body>> {
    let rpc_routes = generate_routes();

    println!("RPC: {req:#?}");

    let request = RpcRequest::from_http(req).await;
    println!("RPC: Message: {request:#?}");

    if let Ok(request) = request {
        return route(shared, hub_channel, request, &rpc_routes).await;
    }

    bail!("Unexpected request")
}
