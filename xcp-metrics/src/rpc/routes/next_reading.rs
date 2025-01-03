//! RPC route for `Plugin.Local.next_reading`.
use std::sync::Arc;

use http::Response;
use xapi::rpc::message::{request::RpcRequest, response::RpcResponse};

use crate::XcpMetricsShared;

#[derive(Clone, Copy, Default)]
pub struct PluginLocalNextReadingRoute;

pub async fn run(
    _shared: Arc<XcpMetricsShared>,
    request: RpcRequest,
) -> anyhow::Result<Response<String>> {
    RpcResponse::respond_to(
        &request, /* next_reading: */ 5.0, /* Same as register */
    )
}
