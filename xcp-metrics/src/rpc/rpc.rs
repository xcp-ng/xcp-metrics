use anyhow::bail;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    xapi::hyper::{body::HttpBody, Body, Request, Response},
    xmlrpc::{parse_method, PluginLocalRegister, XcpRpcMethod},
};

use crate::{
    hub::HubPushMessage,
    providers::{protocol_v2::ProtocolV2Provider, Provider},
};

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
            if let Some(register_rpc) = PluginLocalRegister::try_from_method(method) {
                println!("RPC: {register_rpc:?}");

                // Start protocol v2 provider
                ProtocolV2Provider::new(&register_rpc.uid).start_provider(hub_channel.clone());

                return Ok(Response::builder().body("Working".into())?.into());
            }
        }
    }

    bail!("Not expected")
}
