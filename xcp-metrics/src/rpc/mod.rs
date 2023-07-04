mod rpc;
pub mod xapi;

use futures::future::BoxFuture;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    xapi::hyper::{Body, Response},
    xmlrpc::{
        dxr::{MethodCall, TryFromValue},
        PluginLocalRegister,
    },
};

use crate::{
    hub::HubPushMessage,
    providers::{protocol_v2::ProtocolV2Provider, Provider},
};

pub trait XcpRpcRoute: 'static + Sync + Send {
    fn run(
        &self,
        hub_channel: mpsc::UnboundedSender<HubPushMessage>,
        method: MethodCall,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>>;

    fn make_route() -> Box<dyn XcpRpcRoute>
    where
        Self: Default,
    {
        Box::new(Self::default())
    }
}

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
