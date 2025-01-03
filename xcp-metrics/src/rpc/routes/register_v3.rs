//! RPC route for `Plugin.Metrics.register_v3`.
use std::sync::Arc;

use http::Response;
use xapi::rpc::{
    message::{error::RpcError, request::RpcRequest, response::RpcResponse},
    methods::rrdd::PluginMetricsRegister,
};

use crate::{
    providers::{protocol_v3::ProtocolV3Provider, Provider},
    XcpMetricsShared,
};

pub async fn run(
    shared: Arc<XcpMetricsShared>,
    request: RpcRequest,
) -> anyhow::Result<Response<String>> {
    let register_rpc: PluginMetricsRegister = request
        .clone()
        .try_into_method()
        .ok_or_else(|| anyhow::anyhow!("No value provided"))?;

    if register_rpc.version != "OpenMetrics 1.0.0" {
        return RpcError::respond_to(
            Some(&request),
            -32000,
            "Unsupported OpenMetrics version",
            Some(register_rpc.version),
        );
    }

    if shared // check if plugin exists and is active
        .plugins
        .get(register_rpc.uid.as_str())
        .map(|handle| !handle.is_finished())
        .is_none()
    {
        tracing::info!(uid = register_rpc.uid, "Starting protocol v3 provider");
        let plugin_handle =
            ProtocolV3Provider::new(&register_rpc.uid).start_provider(shared.hub_channel.clone());

        shared
            .plugins
            .insert(register_rpc.uid.into(), plugin_handle);
    } else {
        tracing::warn!(
            "Attempted to register an already registered plugin {}",
            register_rpc.uid
        );
    }

    RpcResponse::respond_to(&request, "OK")
}
