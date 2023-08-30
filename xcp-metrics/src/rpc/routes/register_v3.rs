//! RPC route for `Plugin.Metrics.register_v3`.
use std::sync::Arc;

use futures::future::BoxFuture;
use xapi::{
    hyper::{Body, Response},
    rpc::{
        message::{RpcError, RpcRequest, RpcResponse},
        methods::{PluginLocalRegister, PluginMetricsRegister},
        XcpRpcMethodNamed,
    },
};

use super::XcpRpcRoute;
use crate::{
    providers::{protocol_v3::ProtocolV3Provider, Provider},
    XcpMetricsShared,
};

#[derive(Clone, Copy, Default)]
pub struct PluginMetricsRegisterRoute;

impl XcpRpcRoute for PluginMetricsRegisterRoute {
    fn run(
        &self,
        shared: Arc<XcpMetricsShared>,
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        Box::pin(async move {
            let register_rpc: PluginMetricsRegister = request
                .clone()
                .try_into_method()
                .ok_or_else(|| anyhow::anyhow!("No value provided"))?;

            if register_rpc.protocol != "OpenMetrics 1.0.0" {
                return RpcError::respond_to(
                    Some(&request),
                    -32000,
                    "Unsupported OpenMetrics version",
                    Some(register_rpc.protocol),
                );
            }

            if shared // check if plugin exists and is active
                .plugins
                .get(register_rpc.uid.as_str())
                .map(|handle| !handle.is_finished())
                .is_none()
            {
                tracing::info!(uid = register_rpc.uid, "Starting protocol v3 provider");
                let plugin_handle = ProtocolV3Provider::new(&register_rpc.uid)
                    .start_provider(shared.hub_channel.clone());

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
        })
    }

    fn get_name(&self) -> &'static str {
        PluginLocalRegister::get_method_name()
    }
}
