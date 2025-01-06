//! RPC metrics path.
//!
//! TODO: Deal with disconnected clients.
//!       Track metrics families.

use std::path::Path;

use tokio::{
    net::{UnixListener, UnixStream},
    sync::{mpsc, oneshot},
    task,
};
use xcp_metrics_common::{
    openmetrics::{self, prost::Message},
    protocol::{FetchMetrics, ProtocolMessage, XcpMetricsAsyncStream},
};

use crate::hub::{HubPullResponse, HubPushMessage, PullMetrics};

async fn rpc_session(
    mut stream: UnixStream,
    hub: mpsc::UnboundedSender<HubPushMessage>,
) -> anyhow::Result<()> {
    loop {
        let message = stream.recv_message_async().await?;
        tracing::debug!("Received {message:?}");

        match message {
            ProtocolMessage::CreateFamily(create_family) => {
                hub.send(HubPushMessage::CreateFamily(create_family))?
            }
            ProtocolMessage::RemoveFamily(remove_family) => {
                hub.send(HubPushMessage::RemoveFamily(remove_family))?
            }
            ProtocolMessage::UpdateMetric(update_metric) => {
                hub.send(HubPushMessage::UpdateMetric(update_metric))?
            }
            ProtocolMessage::RemoveMetric(remove_metric) => {
                hub.send(HubPushMessage::RemoveMetric(remove_metric))?
            }

            ProtocolMessage::FetchMetrics(fetch_metrics) => {
                // Get metrics from hub
                let (sender, receiver) = oneshot::channel();
                hub.send(HubPushMessage::PullMetrics(PullMetrics(sender)))?;

                let HubPullResponse::Metrics(metrics_set) = receiver.await?;

                match fetch_metrics {
                    FetchMetrics::OpenMetrics1 => {
                        let mut buffer = String::new();
                        openmetrics::text::write_metrics_set_text(&mut buffer, &metrics_set)?;

                        stream.send_message_raw_async(buffer.as_bytes()).await?;
                    }
                    FetchMetrics::OpenMetrics1Binary => {
                        let buffer =
                            openmetrics::MetricSet::from((*metrics_set).clone()).encode_to_vec();

                        stream.send_message_raw_async(&buffer).await?;
                    }
                }
            }
        }
    }
}

pub async fn run(path: &Path, hub: mpsc::UnboundedSender<HubPushMessage>) -> anyhow::Result<()> {
    let listener = UnixListener::bind(path)?;

    loop {
        let (stream, _) = listener.accept().await?;
        let hub = hub.clone();

        task::spawn(async move {
            if let Err(e) = rpc_session(stream, hub).await {
                tracing::error!("RPC session error: {e}");
            }
        });
    }
}
