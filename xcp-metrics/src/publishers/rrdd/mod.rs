pub mod round_robin;

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::{select, sync::mpsc, task::JoinHandle};
use xcp_metrics_common::rrdd::rrd_updates::{RrdXport, RrdXportInfo};

use crate::hub::{HubPullResponse, HubPushMessage, PullMetrics};

use self::round_robin::RoundRobinBuffer;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RrdEntry {
    /// Full entry name (KIND:owner:uuid:metric_name)
    pub name: Box<str>,

    /// Metrics per five seconds (for a hour)
    pub five_seconds: RoundRobinBuffer<f64>,

    /// Metrics per hour (for a day)
    pub hours: RoundRobinBuffer<f64>,

    /// Metrics per day (for a month)
    pub days: RoundRobinBuffer<f64>,
}

#[derive(Debug)]
pub enum RrddServerMessage {
    RequestRrdUpdates(RrdXportInfo, mpsc::Sender<anyhow::Result<RrdXport>>),
}

#[derive(Debug)]
pub struct RrddServer {
    receiver: mpsc::UnboundedReceiver<RrddServerMessage>,
    host_uuid: uuid::Uuid,
}

impl RrddServer {
    pub fn new() -> (Self, mpsc::UnboundedSender<RrddServerMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        (
            Self {
                receiver,
                host_uuid: uuid::Uuid::new_v4(),
            },
            sender,
        )
    }

    async fn pull_metrics(
        &self,
        hub_channel: &mpsc::UnboundedSender<HubPushMessage>,
    ) -> anyhow::Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        hub_channel.send(HubPushMessage::PullMetrics(PullMetrics(tx)))?;

        let response = rx.recv().await.ok_or(anyhow::anyhow!("No response"))?;

        match response {
            HubPullResponse::Metrics(metrics) => {
                tracing::debug!("TODO {metrics:?}");
            } //r => tracing::error!("Unsupported hub response: {r:?}"),
        }

        Ok(())
    }

    async fn process_message(&self, message: RrddServerMessage) {}

    #[tracing::instrument]
    fn start(mut self, hub_channel: mpsc::UnboundedSender<HubPushMessage>) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(5));

            loop {
                select! {
                    _ = timer.tick() => {
                        tracing::debug!("Pulling metrics");

                        if let Err(e) = self.pull_metrics(&hub_channel).await {
                            tracing::error!("Unable to pull metrics {e}");
                        }
                    },
                    msg = self.receiver.recv() => {
                        match msg {
                            Some(msg) => self.process_message(msg).await,
                            None => tracing::error!("Unable to read channel message")
                        }
                    }
                }
            }
        })
    }
}
