//! RPC route for `Plugin.Local.next_reading`.

use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    rpc::{
        message::{RpcRequest, RpcResponse},
        response::PluginLocalRegisterResponse,
    },
    xapi::hyper::{Body, Response},
};

use crate::{hub::HubPushMessage, rpc::RpcShared};

use super::XcpRpcRoute;

#[derive(Clone, Copy, Default)]
pub struct PluginLocalNextReadingRoute;

impl XcpRpcRoute for PluginLocalNextReadingRoute {
    fn run(
        &self,
        _shared: Arc<RpcShared>,
        _hub_channel: mpsc::UnboundedSender<HubPushMessage>,
        request: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        Box::pin(async move {
            RpcResponse::respond_to(
                &request,
                PluginLocalRegisterResponse {
                    next_reading: 5.0, /* See register. */
                },
            )
        })
    }
}
