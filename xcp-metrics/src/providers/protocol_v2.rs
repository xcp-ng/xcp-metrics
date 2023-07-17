//! Protocol v2 plugin metrics provider
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime},
};

use tokio::{
    fs::File,
    io::AsyncReadExt,
    sync::mpsc,
    task::{self, JoinHandle},
    time,
};
use xcp_metrics_common::{
    metrics::{Metric, MetricPoint, MetricType, MetricValue},
    rrdd::{
        protocol_common::{DataSourceType, DataSourceValue},
        protocol_v2::{RrddMessageHeader, RrddMetadata, RrddMetadataRaw},
    },
};

use crate::hub::{CreateFamily, HubPushMessage, RegisterMetrics, UnregisterMetrics, UpdateMetrics};

use super::Provider;

const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

#[derive(Debug, Clone)]
struct PluginData {
    metadata: RrddMetadata,
    values: Box<[DataSourceValue]>,
    metadata_checksum: u32,
    timestamp: SystemTime,
}

#[derive(Debug, Clone)]
pub struct ProtocolV2Provider {
    name: Box<str>,
    path: PathBuf,
    state: Option<PluginData>,
    registered_metrics: HashMap<Box<str>, uuid::Uuid>,
    hub_channel: Option<mpsc::UnboundedSender<HubPushMessage>>,
    last_reset: SystemTime,
}

impl ProtocolV2Provider {
    pub fn new(plugin_name: &str) -> Self {
        Self {
            name: plugin_name.into(),
            path: Path::new(METRICS_SHM_PATH).join(plugin_name),
            state: None,
            registered_metrics: HashMap::new(),
            hub_channel: None,
            last_reset: SystemTime::now(),
        }
    }

    async fn collect_plugin_metrics(&mut self) -> anyhow::Result<bool> {
        let mut file = File::open(&self.path).await?;
        let header = RrddMessageHeader::parse_async(&mut file).await;
        let mut updated_metadata = false;

        tracing::debug!("Readed {header:?}");

        if let Ok(header) = header {
            // Get the most up to date PluginData.
            let mut data = match self.state.as_ref() {
                /* matching checksums, no need to update metadata */
                Some(
                    data @ &PluginData {
                        metadata_checksum, ..
                    },
                ) if metadata_checksum == header.metadata_checksum => (*data).clone(),

                /* Regenerate data */
                _ => {
                    updated_metadata = true;
                    self.last_reset = SystemTime::now();

                    // Read metadata
                    let mut metadata_string = vec![0u8; header.metadata_length as usize];
                    file.read_exact(&mut metadata_string).await?;
                    let metadata: RrddMetadata =
                        serde_json::from_slice::<RrddMetadataRaw>(&metadata_string)?.try_into()?;

                    PluginData {
                        values: vec![DataSourceValue::Undefined; metadata.datasources.len()]
                            .into_boxed_slice(),
                        metadata,
                        metadata_checksum: header.metadata_checksum,
                        timestamp: header.timestamp,
                    }
                }
            };

            // Update data value slice using raw values in header along with metadata.
            data.values
                .iter_mut()
                .zip(data.metadata.datasources.values())
                .zip(header.values.iter())
                .for_each(|((dest, meta), raw)| {
                    *dest = match meta.value {
                        DataSourceValue::Int64(_) => {
                            DataSourceValue::Int64(i64::from_be_bytes(*raw))
                        }
                        DataSourceValue::Float(_) => {
                            DataSourceValue::Float(f64::from_be_bytes(*raw))
                        }
                        DataSourceValue::Undefined => DataSourceValue::Undefined,
                    }
                });

            data.timestamp = header.timestamp;

            self.state.replace(data);
        }

        Ok(updated_metadata)
    }

    /// Send metrics to hub, registering them if they are not.
    async fn send_values(&mut self, hub_channel: &mpsc::UnboundedSender<HubPushMessage>) {
        let Some(state) = self.state.as_ref() else { return };

        std::iter::zip(state.metadata.datasources.iter(), state.values.iter()).for_each(
            |((name, metadata), value)| {
                // Wrap value into its appropriate MetricPoint.
                let metric_point = MetricPoint::from_protocol_v2(
                    metadata,
                    value,
                    state.timestamp,
                    self.last_reset,
                );

                match self.registered_metrics.get(name) {
                    Some(&uuid) => {
                        // Update metrics values
                        let new_values = vec![metric_point].into_boxed_slice();

                        hub_channel
                            .send(HubPushMessage::UpdateMetrics(UpdateMetrics {
                                new_values,
                                uuid,
                            }))
                            .unwrap();
                    }
                    None => {
                        // Not yet registered, register it.
                        let metric_uuid = uuid::Uuid::new_v4();

                        // Register family
                        hub_channel
                            .send(HubPushMessage::CreateFamily(CreateFamily {
                                name: name.clone(),
                                metric_type: metric_point.value.get_type(),
                                unit: metadata.units.clone(),
                                help: metadata.description.clone(),
                            }))
                            .unwrap();

                        // Register metric
                        hub_channel
                            .send(HubPushMessage::RegisterMetrics(RegisterMetrics {
                                family: name.clone(),
                                uuid: metric_uuid,
                                metrics: Metric::from_protocol_v2(
                                    metadata,
                                    value,
                                    state.timestamp,
                                    self.last_reset,
                                ),
                            }))
                            .unwrap();

                        self.registered_metrics.insert(name.clone(), metric_uuid);
                    }
                };
            },
        );
    }

    fn check_metrics(&mut self, hub_channel: &mpsc::UnboundedSender<HubPushMessage>) {
        let Some(state) = &self.state else { return };

        self.registered_metrics.retain(|key, uuid| {
            // Check if the key exists in the new metadata.
            if !state.metadata.datasources.contains_key(key) {
                // missing: unregister
                hub_channel
                    .send(HubPushMessage::UnregisterMetrics(UnregisterMetrics {
                        uuid: *uuid,
                    }))
                    .ok();

                false
            } else {
                true
            }
        })
    }
}

impl Provider for ProtocolV2Provider {
    fn start_provider(
        mut self,
        hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    ) -> JoinHandle<()> {
        self.hub_channel.replace(hub_channel.clone());

        task::spawn(async move {
            tracing::trace_span!("plugin {}", self.name);

            loop {
                let updated_metadata = self.collect_plugin_metrics().await;

                if let Ok(true) = updated_metadata {
                    tracing::info!("Updated metadata");
                }

                tracing::debug!("New state: {:?}", self.state);

                match updated_metadata {
                    // Check for removed metrics
                    Ok(true) => self.check_metrics(&hub_channel),
                    Ok(false) => (),
                    Err(e) => tracing::error!("{e}"),
                }

                self.send_values(&hub_channel).await;

                time::sleep(Duration::from_secs(5)).await
            }
        })
    }
}

impl Drop for ProtocolV2Provider {
    fn drop(&mut self) {
        // Unregister plugins
        if let Some(hub_channel) = &self.hub_channel {
            self.registered_metrics.iter().for_each(|(name, uuid)| {
                tracing::info!("Unregistering {name}");

                hub_channel
                    .send(HubPushMessage::UnregisterMetrics(UnregisterMetrics {
                        uuid: *uuid,
                    }))
                    .ok(); // ignore failure (destroyed hub ?)
            });
        }
    }
}
