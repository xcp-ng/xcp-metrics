use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    rpc::message::RpcRequest,
    xapi::hyper::{Body, Response},
};

use crate::{hub::HubPushMessage, rpc::RpcShared};

use super::XcpRpcRoute;

#[derive(Clone, Copy, Default)]
pub struct PluginLocalDeregisterRoute;

impl XcpRpcRoute for PluginLocalDeregisterRoute {
    fn run(
        &self,
        shared: Arc<RpcShared>,
        _hub_channel: mpsc::UnboundedSender<HubPushMessage>,
        _request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        todo!()
    }
}
