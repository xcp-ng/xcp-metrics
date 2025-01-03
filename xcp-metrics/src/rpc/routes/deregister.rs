//! RPC route for `Plugin.Local.deregister`.
use std::sync::Arc;

use http::Response;
use xapi::rpc::{
    message::{request::RpcRequest, response::RpcResponse},
    methods::rrdd::PluginLocalDeregister,
};

use crate::XcpMetricsShared;

pub async fn run(
    shared: Arc<XcpMetricsShared>,
    request: RpcRequest,
) -> anyhow::Result<Response<String>> {
    let deregister_rpc: PluginLocalDeregister = request
        .clone()
        .try_into_method()
        .ok_or_else(|| anyhow::anyhow!("No value provided"))?;

    if let Some((name, handle)) = shared.plugins.remove(deregister_rpc.uid.as_str()) {
        tracing::info!("RPC: Unregistered {name}");

        handle.abort();
    }

    RpcResponse::respond_to(&request, "Done")
}
