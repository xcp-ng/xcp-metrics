//! RPC routes
mod deregister;
mod get_formats;
mod next_reading;
mod register;
mod register_v3;

use futures::future::BoxFuture;
use std::{collections::HashMap, sync::Arc};

use xapi::{
    hyper::{Body, Response},
    rpc::{
        message::RpcRequest,
        methods::{
            PluginLocalDeregister, PluginLocalNextReading, PluginLocalRegister,
            PluginMetricsDeregister, PluginMetricsGetVersions, PluginMetricsRegister,
        },
        XcpRpcMethodNamed,
    },
};

use self::{
    deregister::PluginLocalDeregisterRoute, get_formats::PluginMetricsGetVersionsRoute,
    next_reading::PluginLocalNextReadingRoute, register::PluginLocalRegisterRoute,
    register_v3::PluginMetricsRegisterRoute,
};
use crate::{publishers::openmetrics::OpenMetricsRoute, XcpMetricsShared};

pub trait XcpRpcRoute: 'static + Sync + Send {
    fn run(
        &self,
        shared: Arc<XcpMetricsShared>,
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>>;

    fn make_route() -> Box<dyn XcpRpcRoute>
    where
        Self: Default,
    {
        Box::<Self>::default()
    }

    fn get_name(&self) -> &'static str {
        "(Unammed)"
    }
}

impl std::fmt::Debug for dyn XcpRpcRoute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_name())
    }
}

#[derive(Debug)]
pub struct RpcRoutes(HashMap<&'static str, Box<dyn XcpRpcRoute>>);

impl Default for RpcRoutes {
    fn default() -> Self {
        Self(
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
                (
                    PluginLocalNextReading::get_method_name(),
                    PluginLocalNextReadingRoute::make_route(),
                ),
                (
                    PluginMetricsGetVersions::get_method_name(),
                    PluginMetricsGetVersionsRoute::make_route(),
                ),
                (
                    PluginMetricsRegister::get_method_name(),
                    PluginMetricsRegisterRoute::make_route(),
                ),
                (
                    PluginMetricsDeregister::get_method_name(),
                    PluginLocalDeregisterRoute::make_route(),
                ),
            ]
            .into_iter()
            .collect(),
        )
    }
}

impl RpcRoutes {
    pub fn get(&self, name: &str) -> Option<&dyn XcpRpcRoute> {
        self.0.get(name).map(|r| r.as_ref())
    }
}
