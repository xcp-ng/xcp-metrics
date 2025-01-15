//! RPC metrics path.

use std::{collections::HashSet, path::Path};

use compact_str::CompactString;
use tokio::{
    net::{UnixListener, UnixStream},
    sync::{mpsc, oneshot},
    task,
};
use uuid::Uuid;
use xcp_metrics_common::{
    openmetrics::{self, prost::Message},
    protocol::{FetchMetrics, ProtocolMessage, RemoveFamily, RemoveMetric, XcpMetricsAsyncStream},
};

use crate::hub::{HubPullResponse, HubPushMessage, PullMetrics};

struct RpcSessionState {
    // Keep track of all registered metrics and families to unregister them properly if the plugin dies.
    families: HashSet<CompactString>,
    metrics: HashSet<(Uuid, CompactString)>,

    hub: mpsc::UnboundedSender<HubPushMessage>,
    stream: UnixStream,
}

impl RpcSessionState {
    pub async fn run(&mut self) -> anyhow::Result<()> {
        loop {
            let message = self.stream.recv_message_async().await?;

            tracing::debug!("Received {message:?}");
            self.process_message(message).await?;
        }
    }

    pub async fn process_message(&mut self, message: ProtocolMessage) -> anyhow::Result<()> {
        match message {
            ProtocolMessage::CreateFamily(create_family) => {
                if self.families.insert(create_family.name.clone()) {
                    self.hub.send(HubPushMessage::CreateFamily(create_family))?
                } else {
                    tracing::warn!(
                        "{:?} is registering {} twice",
                        self.stream,
                        create_family.name
                    );
                }
            }
            ProtocolMessage::RemoveFamily(remove_family) => {
                if self.families.remove(remove_family.name.as_str()) {
                    self.hub.send(HubPushMessage::RemoveFamily(remove_family))?
                } else {
                    tracing::warn!(
                        "{:?} is trying to remove '{}' but hasn't registered it",
                        self.stream,
                        remove_family.name
                    );
                }
            }
            ProtocolMessage::UpdateMetric(update_metric) => {
                self.metrics
                    .insert((update_metric.uuid, update_metric.family_name.clone()));
                self.hub.send(HubPushMessage::UpdateMetric(update_metric))?
            }
            ProtocolMessage::RemoveMetric(remove_metric) => {
                self.metrics
                    .remove(&(remove_metric.uuid, remove_metric.family_name.clone()));
                self.hub.send(HubPushMessage::RemoveMetric(remove_metric))?
            }

            ProtocolMessage::FetchMetrics(fetch_metrics) => {
                // Get metrics from hub
                let (sender, receiver) = oneshot::channel();
                self.hub
                    .send(HubPushMessage::PullMetrics(PullMetrics(sender)))?;

                let HubPullResponse::Metrics(metrics_set) = receiver.await?;

                match fetch_metrics {
                    FetchMetrics::OpenMetrics1 => {
                        let mut buffer = String::new();
                        openmetrics::text::write_metrics_set_text(&mut buffer, &metrics_set)?;

                        self.stream
                            .send_message_raw_async(buffer.as_bytes())
                            .await?;
                    }
                    FetchMetrics::OpenMetrics1Binary => {
                        let buffer =
                            openmetrics::MetricSet::from((*metrics_set).clone()).encode_to_vec();

                        self.stream.send_message_raw_async(&buffer).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

pub async fn run(path: &Path, hub: mpsc::UnboundedSender<HubPushMessage>) -> anyhow::Result<()> {
    let listener = UnixListener::bind(path)?;

    loop {
        let (stream, _) = listener.accept().await?;
        let hub = hub.clone();

        task::spawn(async move {
            let mut state = RpcSessionState {
                families: HashSet::new(),
                metrics: HashSet::new(),
                hub,
                stream,
            };

            if let Err(e) = state.run().await {
                tracing::debug!("RPC session error: {e}")
            }

            // We need to remove all the families/metrics made by the plugin.
            state.metrics.into_iter().for_each(|(uuid, family_name)| {
                state
                    .hub
                    .send(HubPushMessage::RemoveMetric(RemoveMetric {
                        family_name,
                        uuid,
                    }))
                    .ok();
            });

            state.families.into_iter().for_each(|name| {
                state
                    .hub
                    .send(HubPushMessage::RemoveFamily(RemoveFamily { name }))
                    .ok();
            });
        });
    }
}
