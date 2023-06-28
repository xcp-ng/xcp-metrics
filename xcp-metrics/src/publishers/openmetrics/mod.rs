//! OpenMetrics based metrics publisher
mod convert;

use anyhow::bail;
use async_trait::async_trait;
use prost::Message;
use tokio::sync::mpsc;
use xcp_metrics_common::{metrics::MetricSet, xapi::hyper::Body};

use crate::{
    hub::{HubPullResponse, HubPushMessage, PullMetrics},
    rpc::XcpRpcRoute,
};

use self::convert::openmetrics;

fn generate_openmetrics_message(metrics: MetricSet) -> Vec<u8> {
    openmetrics::MetricSet::from(metrics).encode_to_vec()
}

pub struct OpenMetricsRoute;

#[async_trait]
impl XcpRpcRoute for OpenMetricsRoute {
    async fn run(hub_channel: mpsc::UnboundedSender<HubPushMessage>) -> anyhow::Result<Body> {
        let (sender, mut receiver) = mpsc::unbounded_channel();

        hub_channel.send(HubPushMessage::PullMetrics(PullMetrics(sender)))?;

        let Some(HubPullResponse::Metrics(metrics)) = receiver.recv().await else {
        bail!("Unable to fetch metrics from hub")
      };

        let message = generate_openmetrics_message((*metrics).clone());

        Ok(message.into())
    }
}
