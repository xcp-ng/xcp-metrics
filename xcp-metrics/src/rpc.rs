//! RPC metrics path.

use std::{collections::HashSet, path::Path};

use compact_str::CompactString;
use flume::Sender;
use smol::{
    net::unix::{UnixListener, UnixStream},
    Executor,
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

    hub: Sender<HubPushMessage>,
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
                    self.hub
                        .send_async(HubPushMessage::CreateFamily(create_family))
                        .await?
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
                    self.hub
                        .send_async(HubPushMessage::RemoveFamily(remove_family))
                        .await?
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
                self.hub
                    .send_async(HubPushMessage::UpdateMetric(update_metric))
                    .await?
            }
            ProtocolMessage::RemoveMetric(remove_metric) => {
                self.metrics
                    .remove(&(remove_metric.uuid, remove_metric.family_name.clone()));
                self.hub
                    .send_async(HubPushMessage::RemoveMetric(remove_metric))
                    .await?
            }

            ProtocolMessage::FetchMetrics(fetch_metrics) => {
                // Get metrics from hub
                let (sender, receiver) = flume::bounded(0);
                self.hub
                    .send_async(HubPushMessage::PullMetrics(PullMetrics(sender)))
                    .await?;

                let HubPullResponse::Metrics(metrics_set) = receiver.recv_async().await?;

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

async fn rpc_session(stream: UnixStream, hub: Sender<HubPushMessage>) {
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
}

pub async fn run(path: &Path, hub: Sender<HubPushMessage>) -> anyhow::Result<()> {
    let listener = UnixListener::bind(path)?;
    let executor = Executor::new();

    executor
        .run(async {
            loop {
                let (stream, _) = listener.accept().await?;
                let hub = hub.clone();

                executor.spawn(rpc_session(stream, hub)).detach();
            }
        })
        .await
}
