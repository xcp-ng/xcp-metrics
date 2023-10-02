//! Protocol v3 plugin metrics provider

use std::{
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use tokio::{fs::File, sync::mpsc, task::JoinHandle, time};
use xcp_metrics_common::{metrics::MetricSet, protocol_v3, utils::delta::MetricSetModel};

use crate::hub::{CreateFamily, HubPushMessage, RegisterMetrics, UnregisterMetrics, UpdateMetrics};

use super::Provider;

const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

#[derive(Debug, Clone)]
pub struct ProtocolV3Provider {
    name: Box<str>,
    path: PathBuf,
    last_timestamp: SystemTime,

    model: MetricSetModel,
}

impl ProtocolV3Provider {
    pub fn new(plugin_name: &str) -> Self {
        Self {
            name: plugin_name.into(),
            path: Path::new(METRICS_SHM_PATH).join(plugin_name),
            last_timestamp: SystemTime::now(),
            model: MetricSetModel::default(),
        }
    }

    async fn fetch_protocol_v3(&self) -> anyhow::Result<Option<MetricSet>> {
        let mut file = File::open(&self.path).await?;

        // Read metrics
        let (header, metrics) = protocol_v3::parse_v3_async(&mut file).await?;

        if header.timestamp == self.last_timestamp {
            tracing::debug!("Metrics have not been updated");
            return Ok(None);
        }

        Ok(Some(metrics))
    }
}

impl Provider for ProtocolV3Provider {
    fn start_provider(
        mut self,
        hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    ) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            tracing::debug_span!("Plugin {}", self.name);

            loop {
                match self.fetch_protocol_v3().await {
                    Ok(Some(new_metrics)) => {
                        let delta = self.model.compute_delta(&new_metrics);

                        // Update model
                        self.model.apply_delta(&delta);

                        // Remove metrics
                        delta.removed_metrics.into_iter().for_each(|uuid| {
                            if let Err(e) = hub_channel.send(HubPushMessage::UnregisterMetrics(
                                UnregisterMetrics { uuid },
                            )) {
                                tracing::error!("Unregister error {e}");
                            }
                        });

                        // Add new families
                        delta.added_families.into_iter().for_each(|(name, family)| {
                            if let Err(e) =
                                hub_channel.send(HubPushMessage::CreateFamily(CreateFamily {
                                    name: name.into(),
                                    metric_type: family.metric_type,
                                    unit: family.unit.clone(),
                                    help: family.help.clone(),
                                }))
                            {
                                tracing::error!("Register error {e}");
                            }
                        });

                        // Add new metrics
                        delta
                            .added_metrics
                            .into_iter()
                            .for_each(|(family, metrics, uuid)| {
                                if let Err(e) = hub_channel.send(HubPushMessage::RegisterMetrics(
                                    RegisterMetrics {
                                        family: family.into(),
                                        metrics: (*metrics).clone(),
                                        uuid,
                                    },
                                )) {
                                    tracing::error!("Unregister error {e}");
                                }
                            });

                        // Update all metrics
                        new_metrics.families.into_iter().for_each(|(name, family)| {
                            family.metrics.into_iter().for_each(|(_, metric)| {
                                let uuid = self.model.metrics_map[&(name.clone(), metric.labels)];

                                if let Err(e) =
                                    hub_channel.send(HubPushMessage::UpdateMetrics(UpdateMetrics {
                                        uuid,
                                        new_values: metric.metrics_point.clone(),
                                    }))
                                {
                                    tracing::error!("Update error {e}");
                                }
                            });
                        });
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::warn!("Unable to fetch metrics: {e}")
                    }
                }

                time::sleep(Duration::from_secs(5)).await
            }
        })
    }
}
