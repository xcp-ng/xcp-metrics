//! OpenMetrics based metrics publisher
mod convert;

use anyhow::bail;
use futures::future::BoxFuture;
use prost::Message;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    metrics::MetricSet,
    xapi::hyper::{Body, Response},
    xmlrpc::dxr::MethodCall,
};

use crate::{
    hub::{HubPullResponse, HubPushMessage, PullMetrics},
    rpc::routes::XcpRpcRoute,
};

use self::convert::openmetrics;

fn generate_openmetrics_message(metrics: MetricSet) -> Vec<u8> {
    openmetrics::MetricSet::from(metrics).encode_to_vec()
}

#[derive(Copy, Clone, Default)]
pub struct OpenMetricsRoute;

impl XcpRpcRoute for OpenMetricsRoute {
    fn run(
        &self,
        hub_channel: mpsc::UnboundedSender<HubPushMessage>,
        _method: MethodCall,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        println!("RPC: Open Metrics query");

        Box::pin(async move {
            let (sender, mut receiver) = mpsc::unbounded_channel();

            hub_channel.send(HubPushMessage::PullMetrics(PullMetrics(sender)))?;

            let Some(HubPullResponse::Metrics(metrics)) = receiver.recv().await else {
                bail!("Unable to fetch metrics from hub")
            };

            let message = generate_openmetrics_message((*metrics).clone());

            Ok(Response::builder()
                .header("content-type", "application/x-protobuf")
                .body(message.into())?)
        })
    }
}
