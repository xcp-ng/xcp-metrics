//! RPC route for `Plugin.Local.next_reading`.

use std::sync::Arc;

use futures::future::BoxFuture;
use xapi::{
    hyper::{Body, Response},
    rpc::message::{RpcRequest, RpcResponse},
};

use super::XcpRpcRoute;
use crate::XcpMetricsShared;

#[derive(Clone, Copy, Default)]
pub struct PluginLocalNextReadingRoute;

impl XcpRpcRoute for PluginLocalNextReadingRoute {
    fn run(
        &self,
        _shared: Arc<XcpMetricsShared>,
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        Box::pin(async move {
            RpcResponse::respond_to(
                &request, /* next_reading: */ 5.0, /* Same as register */
            )
        })
    }
}
