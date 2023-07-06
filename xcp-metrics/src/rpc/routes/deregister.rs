use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    rpc::{message::{RpcRequest, RpcResponse}, methods::PluginLocalDeregister},
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
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        Box::pin(async move {
            let deregister_rpc: PluginLocalDeregister = request
                .clone()
                .try_into_method()
                .ok_or_else(|| anyhow::anyhow!("No value provided"))?;

            if let Some((name, handle)) = shared.plugins.remove(deregister_rpc.uid.as_str()) {
                println!("RPC: Unregistered {name}");

                handle.abort();
            }

            RpcResponse::respond_to(&request, "Done")
        })
    }
}
