//! RPC route for `Plugin.Metrics.get_formats`.
use std::sync::Arc;

use http::Response;
use xapi::rpc::{
    message::{request::RpcRequest, response::RpcResponse},
    response::rrdd::PluginMetricsVersionsResponse,
};

use crate::XcpMetricsShared;

pub async fn run(
    _shared: Arc<XcpMetricsShared>,
    request: RpcRequest,
) -> anyhow::Result<Response<String>> {
    RpcResponse::respond_to(
        &request,
        PluginMetricsVersionsResponse {
            versions: vec!["OpenMetrics 1.0.0".to_string()],
        },
    )
}
