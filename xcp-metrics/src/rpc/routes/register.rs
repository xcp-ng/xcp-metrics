use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    rpc::{message::RpcRequest, methods::PluginLocalRegister},
    xapi::hyper::{Body, Response},
};

use crate::{
    hub::HubPushMessage,
    providers::{protocol_v2::ProtocolV2Provider, Provider},
    rpc::RpcShared,
};

use super::XcpRpcRoute;

#[derive(Clone, Copy, Default)]
pub struct PluginLocalRegisterRoute;

impl XcpRpcRoute for PluginLocalRegisterRoute {
    fn run(
        &self,
        shared: Arc<RpcShared>,
        hub_channel: mpsc::UnboundedSender<HubPushMessage>,
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        Box::pin(async move {
            let register_rpc: PluginLocalRegister = request
                .try_into_method()
                .ok_or_else(|| anyhow::anyhow!("No value provided"))?;

            if !shared.plugins.contains(register_rpc.uid.as_str()) {
                ProtocolV2Provider::new(&register_rpc.uid).start_provider(hub_channel.clone());
                shared.plugins.insert(register_rpc.uid.into());
            } else {
                println!("RPC: Plugin already registered");
            }

            Ok(Response::builder().body("Working".into())?)
        })
    }
}
