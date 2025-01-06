/*!
# Metrics Hub

This is the part that centralize the metrics from the main daemon.
It communicates metrics with [crate::publishers] (pull) and [crate::providers] (push) using
a [`oneshot::Sender<HubPullResponse>`].

All metrics are uniquely identified using a [uuid::Uuid] to ease updating, this identifier
must be generated by the provider (using [uuid::Uuid::new_v4]).
*/
use std::{collections::HashMap, sync::Arc};

use tokio::{
    sync::{mpsc, oneshot},
    task::{self, JoinHandle},
};
use xcp_metrics_common::{
    metrics::{MetricFamily, MetricSet},
    protocol::{CreateFamily, RemoveFamily, RemoveMetric, UpdateMetric},
};

/// Fetch metrics, receiving them in a provided [`oneshot::Sender<HubPullResponse>`].
#[derive(Debug)]
pub struct PullMetrics(pub oneshot::Sender<HubPullResponse>);

/// A message that can be sent to the hub.
#[derive(Debug)]
pub enum HubPushMessage {
    // xcp-metrics protocol messages
    CreateFamily(CreateFamily),
    RemoveFamily(RemoveFamily),
    UpdateMetric(UpdateMetric),
    RemoveMetric(RemoveMetric),

    // Hub-specific messages
    PullMetrics(PullMetrics),
}

/// A hub response.
#[derive(Debug, Clone)]
pub enum HubPullResponse {
    Metrics(Arc<MetricSet>),
}

/// Metrics Hub
#[derive(Debug, Clone, Default)]
pub struct MetricsHub {
    metrics: Arc<MetricSet>,
}

impl MetricsHub {
    /// Starts the Metrics Hub in a new [tokio::task], giving the associated [JoinHandle]
    /// and hub channel ([`mpsc::UnboundedSender<HubPushMessage>`]).
    pub async fn start(self) -> (JoinHandle<()>, mpsc::UnboundedSender<HubPushMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let rendez_vous = oneshot::channel();

        let handle = task::spawn(async move { self.run(receiver, rendez_vous.0).await });

        rendez_vous.1.await.unwrap();

        tracing::info!("Hub ready");

        (handle, sender)
    }

    async fn run(
        mut self,
        mut receiver: mpsc::UnboundedReceiver<HubPushMessage>,
        rendez_vous: oneshot::Sender<()>,
    ) {
        rendez_vous.send(()).ok();

        while let Some(msg) = receiver.recv().await {
            match msg {
                HubPushMessage::CreateFamily(message) => self.create_family(message).await,
                HubPushMessage::RemoveFamily(message) => self.remove_family(message).await,
                HubPushMessage::UpdateMetric(message) => self.update_metric(message).await,
                HubPushMessage::RemoveMetric(message) => self.remove_metric(message).await,
                HubPushMessage::PullMetrics(message) => self.pull_metrics(message).await,
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn create_family(
        &mut self,
        CreateFamily {
            name,
            metric_type,
            unit,
            help,
        }: CreateFamily,
    ) {
        let metrics = Arc::make_mut(&mut self.metrics);

        if let Some(previous_family) = metrics.families.get_mut(&name) {
            // Check for conflicts
            if previous_family.metric_type != metric_type
                || previous_family.unit != unit
                || previous_family.help != help
            {
                tracing::warn!("Incompatible families {previous_family:?} != (type={metric_type}, unit={unit}, help={help})");
            }

            previous_family.reference_count += 1;
        } else {
            // No existing family.
            metrics.families.insert(
                name,
                MetricFamily {
                    reference_count: 1,
                    metric_type,
                    unit: unit.clone(),
                    help: help.clone(),
                    metrics: HashMap::default(),
                },
            );
        }
    }

    #[tracing::instrument(skip(self))]
    async fn remove_family(&mut self, RemoveFamily { name }: RemoveFamily) {
        let metrics = Arc::make_mut(&mut self.metrics);

        let Some(family) = metrics.families.get_mut(&name) else {
            tracing::warn!("Tried to remove missing family {name}");
            return;
        };

        family.reference_count -= 1;

        if family.reference_count == 0 {
            metrics.families.remove(&name);
        }
    }

    #[tracing::instrument(skip(self))]
    async fn remove_metric(&mut self, RemoveMetric { family_name, uuid }: RemoveMetric) {
        let metrics = Arc::make_mut(&mut self.metrics);

        let Some(family) = metrics.families.get_mut(&family_name) else {
            tracing::warn!("Missing family '{family_name}'");
            return;
        };

        if family.metrics.remove(&uuid).is_none() {
            tracing::warn!("Tried to remove missing metric '{family_name}:{uuid}'");
        }
    }

    #[tracing::instrument(skip(self))]
    async fn update_metric(
        &mut self,
        UpdateMetric {
            family_name,
            metric,
            uuid,
        }: UpdateMetric,
    ) {
        let metrics = Arc::make_mut(&mut self.metrics);

        let Some(family) = metrics.families.get_mut(&family_name) else {
            tracing::warn!("Missing family '{family_name}'");
            return;
        };

        family.metrics.insert(uuid, metric);
    }

    #[tracing::instrument(skip(self))]
    async fn pull_metrics(&mut self, message: PullMetrics) {
        let sender = message.0;
        tracing::debug!("Pulling metrics");

        if let Err(e) = sender.send(HubPullResponse::Metrics(Arc::clone(&self.metrics))) {
            tracing::error!("Error occured while sending metrics {e:?}");
        }
    }
}
