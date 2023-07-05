mod deregister;
mod register;

use std::collections::HashMap;

use crate::{hub::HubPushMessage, publishers::openmetrics::OpenMetricsRoute};
use futures::future::BoxFuture;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    rpc::{
        message::RpcRequest,
        methods::{PluginLocalDeregister, PluginLocalRegister},
        XcpRpcMethodNamed,
    },
    xapi::hyper::{Body, Response},
};

use self::{deregister::PluginLocalDeregisterRoute, register::PluginLocalRegisterRoute};

pub trait XcpRpcRoute: 'static + Sync + Send {
    fn run(
        &self,
        hub_channel: mpsc::UnboundedSender<HubPushMessage>,
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>>;

    fn make_route() -> Box<dyn XcpRpcRoute>
    where
        Self: Default,
    {
        Box::<Self>::default()
    }
}

pub fn generate_routes() -> HashMap<&'static str, Box<dyn XcpRpcRoute>> {
    [
        ("OpenMetrics", OpenMetricsRoute::make_route()),
        (
            PluginLocalRegister::get_method_name(),
            PluginLocalRegisterRoute::make_route(),
        ),
        (
            PluginLocalDeregister::get_method_name(),
            PluginLocalDeregisterRoute::make_route(),
        ),
    ]
    .into_iter()
    .collect()
}
