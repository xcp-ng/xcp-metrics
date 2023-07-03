use anyhow::bail;

use tokio::sync::mpsc;
use xcp_metrics_common::{
    xapi::hyper::{body::HttpBody, Body, Request, Response},
    xmlrpc::{dxr::MethodCall, parse_method, PluginLocalRegister, XcpRpcMethod, XcpRpcMethodNamed},
};

use crate::{
    hub::HubPushMessage,
    providers::{protocol_v2::ProtocolV2Provider, Provider},
    publishers::openmetrics::OpenMetricsRoute,
    rpc::XcpRpcRoute,
};

pub async fn rpc_route(
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    method: MethodCall,
) -> anyhow::Result<Response<Body>> {
    // TODO: It should be made using a once_cell+HashMap instead, but it's not easy due to async traits vs Sync/Send traits.
    // TOOD: Rework this

    if method.name() == PluginLocalRegister::get_method_name() {
        return if let Some(register_rpc) = PluginLocalRegister::try_from_method(method) {
            println!("RPC: {register_rpc:?}");

            // Start protocol v2 provider
            ProtocolV2Provider::new(&register_rpc.uid).start_provider(hub_channel.clone());

            Ok(Response::builder().body("Working".into())?)
        } else {
            Ok(Response::builder().body("Invalid register RPC".into())?)
        };
    }

    if method.name() == "OpenMetrics" {
        println!("RPC: Open Metrics query");

        let body = OpenMetricsRoute::run(hub_channel.clone()).await?;

        return Ok(Response::builder().body(body)?);
    }

    Ok(Response::builder()
        .body("Unknown RPC method".into())?
        .into())
}

pub async fn rpc_entrypoint(
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    req: Request<Body>,
) -> anyhow::Result<Response<Body>> {
    println!("RPC: {req:#?}");

    if let Some(Ok(bytes)) = req.into_body().data().await {
        let buffer = bytes.to_vec();
        let str = String::from_utf8_lossy(&buffer);

        let method = parse_method(&str);
        println!("RPC: {method:#?}");

        if let Ok(method) = method {
            return rpc_route(hub_channel, method).await;
        }
    }

    bail!("Unexpected request")
}
