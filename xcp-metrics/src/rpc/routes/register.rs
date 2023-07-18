//! RPC route for `Plugin.Local.register`.
use std::sync::Arc;

use futures::future::BoxFuture;
use xcp_metrics_common::{
    rpc::{
        message::{RpcError, RpcRequest, RpcResponse},
        methods::PluginLocalRegister,
        XcpRpcMethodNamed,
    },
    xapi::hyper::{Body, Response},
};

use super::XcpRpcRoute;
use crate::{
    providers::{protocol_v2::ProtocolV2Provider, Provider},
    XcpMetricsShared,
};

#[derive(Clone, Copy, Default)]
pub struct PluginLocalRegisterRoute;

impl XcpRpcRoute for PluginLocalRegisterRoute {
    fn run(
        &self,
        shared: Arc<XcpMetricsShared>,
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        Box::pin(async move {
            let register_rpc: PluginLocalRegister = request
                .clone()
                .try_into_method()
                .ok_or_else(|| anyhow::anyhow!("No value provided"))?;

            if shared // check if plugin exists and is active
                .plugins
                .get(register_rpc.uid.as_str())
                .map(|handle| !handle.is_finished())
                .is_none()
            {
                let sender = ProtocolV2Provider::new(&register_rpc.uid)
                    .start_provider(shared.hub_channel.clone());

                shared.plugins.insert(register_rpc.uid.into(), sender);

                RpcResponse::respond_to(
                    &request,
                    /* next_reading: */
                    5.0, /* all provider readings are independant, thus this is always 5 */
                )
            } else {
                RpcError::respond_to::<()>(
                    Some(&request),
                    -32000,
                    "Plugin already registered",
                    None,
                )
            }
        })
    }

    fn get_name(&self) -> &'static str {
        PluginLocalRegister::get_method_name()
    }
}
