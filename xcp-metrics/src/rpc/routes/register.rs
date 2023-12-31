//! RPC route for `Plugin.Local.register`.
use std::sync::Arc;

use futures::future::BoxFuture;
use xapi::{
    hyper::{Body, Response},
    rpc::{
        message::{RpcRequest, RpcResponse},
        methods::PluginLocalRegister,
        XcpRpcMethodNamed,
    },
};

use super::XcpRpcRoute;
use crate::{
    providers::{protocol_v2::ProtocolV2Provider, protocol_v3::ProtocolV3Provider, Provider},
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
                let plugin_handle = match register_rpc.protocol.as_str() {
                    "V2" => {
                        tracing::info!(uid = register_rpc.uid, "Starting protocol v2 provider");
                        ProtocolV2Provider::new(&register_rpc.uid)
                            .start_provider(shared.hub_channel.clone())
                    }
                    "V3" => {
                        tracing::info!(uid = register_rpc.uid, "Starting protocol v3 provider");
                        ProtocolV3Provider::new(&register_rpc.uid)
                            .start_provider(shared.hub_channel.clone())
                    }
                    _ => {
                        anyhow::bail!("Unknown or unsupported protocol {}", register_rpc.protocol);
                    }
                };

                shared
                    .plugins
                    .insert(register_rpc.uid.into(), plugin_handle);
            } else {
                tracing::warn!(
                    "Attempted to register an already registered plugin {}",
                    register_rpc.uid
                );
            }

            RpcResponse::respond_to(
                &request,
                /* next_reading: */
                5.0, /* all provider readings are independant, thus this is always 5 */
            )
        })
    }

    fn get_name(&self) -> &'static str {
        PluginLocalRegister::get_method_name()
    }
}
