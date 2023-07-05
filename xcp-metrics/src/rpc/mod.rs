pub mod routes;
pub mod xapi;

use std::collections::HashMap;

use anyhow::bail;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    rpc::{dxr::MethodCall, parse_method_xmlrpc},
    xapi::hyper::{body::HttpBody, http::HeaderValue, Body, Request, Response},
};

use crate::{hub::HubPushMessage, rpc::routes::generate_routes};

use self::routes::XcpRpcRoute;

pub async fn route(
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

pub async fn entrypoint(
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    req: Request<Body>,
) -> anyhow::Result<Response<Body>> {
    let rpc_routes = generate_routes();

    println!("RPC: {req:#?}");

    match req
        .headers()
        .get("content-type")
        .map(|header| header.to_str())
    {
        Some(Ok(s)) => println!("RPC: Content-Type: {s}"),
        _ => (),
    }

    if let Some(Ok(bytes)) = req.into_body().data().await {
        let buffer = bytes.to_vec();
        let str = String::from_utf8_lossy(&buffer);

        let method = parse_method_xmlrpc(&str);
        println!("RPC: {method:#?}");

        if let Ok(method) = method {
            return route(hub_channel, method, &rpc_routes).await;
        }
    }

    bail!("Unexpected request")
}
