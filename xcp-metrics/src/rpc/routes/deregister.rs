use futures::future::BoxFuture;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    xapi::hyper::{Body, Response},
    xmlrpc::dxr::MethodCall,
};

use crate::hub::HubPushMessage;

use super::XcpRpcRoute;

#[derive(Clone, Copy, Default)]
pub struct PluginLocalDeregisterRoute;

impl XcpRpcRoute for PluginLocalDeregisterRoute {
    fn run(
        &self,
        hub_channel: mpsc::UnboundedSender<HubPushMessage>,
        method: MethodCall,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        todo!()
    }
}
