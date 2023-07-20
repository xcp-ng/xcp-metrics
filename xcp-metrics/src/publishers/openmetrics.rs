//! OpenMetrics based metrics publisher
use std::sync::Arc;

use futures::future::BoxFuture;
use tokio::sync::mpsc;
use xcp_metrics_common::{
    metrics::MetricSet,
    openmetrics::{self, prost::Message, text},
    rpc::{message::RpcRequest, methods::OpenMetricsMethod, XcpRpcMethodNamed},
    xapi::hyper::{Body, Response},
};

use crate::{
    hub::{HubPullResponse, HubPushMessage, PullMetrics},
    rpc::routes::XcpRpcRoute,
    XcpMetricsShared,
};

fn generate_openmetrics_message(metrics: MetricSet) -> Vec<u8> {
    openmetrics::MetricSet::from(metrics).encode_to_vec()
}

fn generate_openmetrics_text_message(metrics: MetricSet) -> Vec<u8> {
    let mut output = String::new();

    text::write_metrics_set_text(&mut output, &metrics).unwrap();

    output.into_bytes()
}

const OPENMETRICS_TEXT_CONTENT_TYPE: &str =
    "application/openmetrics-text; version=1.0.0; charset=utf-8";
const OPENMETRICS_PROTOBUF_CONTENT_TYPE: &str = "application/openmetrics-protobuf; version=1.0.0";

#[derive(Copy, Clone, Default)]
pub struct OpenMetricsRoute;

impl XcpRpcRoute for OpenMetricsRoute {
    fn run(
        &self,
        shared: Arc<XcpMetricsShared>,
        message: RpcRequest,
    ) -> BoxFuture<'static, anyhow::Result<Response<Body>>> {
        tracing::info_span!("Open Metrics query");
        tracing::debug!("Preparing query");

        Box::pin(async move {
            let use_protobuf = message
                .try_into_method::<OpenMetricsMethod>()
                .map_or(false, |method| method.protobuf);

            let (sender, mut receiver) = mpsc::unbounded_channel();

            shared
                .hub_channel
                .send(HubPushMessage::PullMetrics(PullMetrics(sender)))?;

            let Some(HubPullResponse::Metrics(metrics)) = receiver.recv().await else {
                anyhow::bail!("Unable to fetch metrics from hub")
            };

            if use_protobuf {
                let message = generate_openmetrics_message((*metrics).clone());

                Ok(Response::builder()
                    .header("content-type", OPENMETRICS_PROTOBUF_CONTENT_TYPE)
                    .body(message.into())?)
            } else {
                let message = generate_openmetrics_text_message((*metrics).clone());

                Ok(Response::builder()
                    .header("content-type", OPENMETRICS_TEXT_CONTENT_TYPE)
                    .body(message.into())?)
            }
        })
    }

    fn get_name(&self) -> &'static str {
        OpenMetricsMethod::get_method_name()
    }
}
