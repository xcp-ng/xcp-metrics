use futures::future::BoxFuture;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    rpc::{
        dxr::{MethodCall, TryFromValue},
        methods::PluginLocalRegister,
    },
    xapi::hyper::{Body, Response},
};

use crate::{
    hub::HubPushMessage,
    providers::{protocol_v2::ProtocolV2Provider, Provider},
};

use super::XcpRpcRoute;

#[derive(Clone, Copy, Default)]
pub struct PluginLocalRegisterRoute;

impl XcpRpcRoute for PluginLocalRegisterRoute {
    fn run(
        &self,
        hub_channel: mpsc::UnboundedSender<HubPushMessage>,
        method: MethodCall,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        Box::pin(async move {
            let register_rpc = PluginLocalRegister::try_from_value(
                method
                    .params()
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("No value provided"))?,
            )?;

            ProtocolV2Provider::new(&register_rpc.uid).start_provider(hub_channel.clone());

            Ok(Response::builder().body("Working".into())?)
        })
    }
}
