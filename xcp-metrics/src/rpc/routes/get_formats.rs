//! RPC route for `Plugin.Metrics.get_formats`.
use std::sync::Arc;

use futures::future::BoxFuture;
use xapi::{
    hyper::{Body, Response},
    rpc::{
        message::{RpcRequest, RpcResponse},
        methods::PluginMetricsGetVersions,
        response::PluginMetricsVersionsResponse,
        XcpRpcMethodNamed,
    },
};

use super::XcpRpcRoute;
use crate::XcpMetricsShared;

#[derive(Default)]
pub struct PluginMetricsGetVersionsRoute;

impl XcpRpcRoute for PluginMetricsGetVersionsRoute {
    fn run(
        &self,
        _shared: Arc<XcpMetricsShared>,
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        Box::pin(async move {
            RpcResponse::respond_to(
                &request,
                PluginMetricsVersionsResponse {
                    versions: vec!["OpenMetrics 1.0.0".to_string()],
                },
            )
        })
    }

    fn get_name(&self) -> &'static str {
        PluginMetricsGetVersions::get_method_name()
    }
}
