//! RPC route for `Plugin.Local.deregister`.
use std::sync::Arc;

use futures::future::BoxFuture;
use xapi::{
    hyper::{Body, Response},
    rpc::{
        message::{RpcRequest, RpcResponse},
        methods::PluginLocalDeregister,
        XcpRpcMethodNamed,
    },
};

use super::XcpRpcRoute;
use crate::XcpMetricsShared;

#[derive(Clone, Copy, Default)]
pub struct PluginLocalDeregisterRoute;

impl XcpRpcRoute for PluginLocalDeregisterRoute {
    fn run(
        &self,
        shared: Arc<XcpMetricsShared>,
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        Box::pin(async move {
            let deregister_rpc: PluginLocalDeregister = request
                .clone()
                .try_into_method()
                .ok_or_else(|| anyhow::anyhow!("No value provided"))?;

            if let Some((name, handle)) = shared.plugins.remove(deregister_rpc.uid.as_str()) {
                tracing::info!("RPC: Unregistered {name}");

                handle.abort();
            }

            RpcResponse::respond_to(&request, "Done")
        })
    }

    fn get_name(&self) -> &'static str {
        PluginLocalDeregister::get_method_name()
    }
}
