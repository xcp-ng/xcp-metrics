use std::collections::HashMap;

use anyhow::bail;

use tokio::sync::mpsc;
use xcp_metrics_common::{
    xapi::hyper::{body::HttpBody, Body, Request, Response},
    xmlrpc::{dxr::MethodCall, parse_method, PluginLocalRegister, XcpRpcMethodNamed},
};

use crate::{hub::HubPushMessage, publishers::openmetrics::OpenMetricsRoute, rpc::XcpRpcRoute};

use super::PluginLocalRegisterRoute;

pub fn generate_routes() -> HashMap<&'static str, Box<dyn XcpRpcRoute>> {
    [
        ("OpenMetrics", OpenMetricsRoute::make_route()),
        (
            PluginLocalRegister::get_method_name(),
            PluginLocalRegisterRoute::make_route(),
        ),
    ].into_iter()
    .collect()
}

pub async fn rpc_route(
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    method: MethodCall,
    rpc_routes: &HashMap<&str, Box<dyn XcpRpcRoute>>,
) -> anyhow::Result<Response<Body>> {
    println!("RPC: {method:?}");

    if let Some(route) = rpc_routes.get(method.name()) {
        route.run(hub_channel, method).await
    } else {
        Ok(Response::builder()
            .body("Unknown RPC method".into())?
            .into())
    }
}

pub async fn rpc_entrypoint(
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    req: Request<Body>,
) -> anyhow::Result<Response<Body>> {
    let rpc_routes = generate_routes();

    println!("RPC: {req:#?}");

    if let Some(Ok(bytes)) = req.into_body().data().await {
        let buffer = bytes.to_vec();
        let str = String::from_utf8_lossy(&buffer);

        let method = parse_method(&str);
        println!("RPC: {method:#?}");

        if let Ok(method) = method {
            return rpc_route(hub_channel, method, &rpc_routes).await;
        }
    }

    bail!("Unexpected request")
}
