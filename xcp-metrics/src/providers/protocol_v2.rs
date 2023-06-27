//! Protocol v2 plugin provider
use std::{
    collections::HashMap,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use tokio::{
    sync::mpsc,
    task::{self, AbortHandle},
    time,
};
use xcp_metrics_common::{
    metrics::{Metric, MetricPoint, MetricValue},
    rrdd::{
        protocol_common::DataSourceValue,
        protocol_v2::{RrddMessageHeader, RrddMetadata, RrddMetadataRaw},
    },
};

use crate::hub::{HubPushMessage, RegisterMetrics, UpdateMetrics};

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
}

impl ProtocolV2Provider {
    pub fn new(plugin_name: &str) -> Self {
        Self {
            name: plugin_name.into(),
            path: Path::new(METRICS_SHM_PATH).join(plugin_name),
            state: None,
            registered_metrics: HashMap::new(),
        }
    }

    // TODO: This is blocking, port it to async.
    fn collect_plugin_metrics(&mut self) -> anyhow::Result<bool> {
        let mut file = std::fs::File::open(&self.path)?;
        let header = RrddMessageHeader::parse_from(&mut file);
        let mut updated_metadata = false;

        println!("{}: Readed {header:?}", self.name);

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
                    println!("{}: Update metadata", self.name);
                    updated_metadata = true;

                    // Read metadata
                    let mut metadata_string = vec![0u8; header.metadata_length as usize];
                    file.read_exact(&mut metadata_string)?;
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

            self.state.replace(data);
        }

        Ok(updated_metadata)
    }

    /// Send metrics to hub, registering them if they are not.
    async fn send_values(&mut self, hub_channel: &mpsc::UnboundedSender<HubPushMessage>) {
        let Some(state) = self.state.as_ref() else { return };

        std::iter::zip(state.metadata.datasources.iter(), state.values.iter()).for_each(
            |((name, metadata), value)| {
                match self.registered_metrics.get(name) {
                    Some(uuid) => {
                        // Update metrics values
                        let new_values = vec![MetricPoint {
                            timestamp: state.timestamp,
                            // TODO: Consider other metric kind
                            value: MetricValue::Gauge(value.into()),
                        }]
                        .into_boxed_slice();

                        hub_channel
                            .send(HubPushMessage::UpdateMetrics(UpdateMetrics {
                                new_values,
                                uuid: *uuid,
                            }))
                            .unwrap();
                    }
                    None => {
                        // Not yet registered, register it.
                        let metric_uuid = uuid::Uuid::new_v4();

                        // Register metric
                        hub_channel
                            .send(HubPushMessage::RegisterMetrics(RegisterMetrics {
                                family: name.clone(),
                                metrics: Metric::from((metadata, value)),
                                uuid: metric_uuid,
                            }))
                            .unwrap();

                        self.registered_metrics.insert(name.clone(), metric_uuid);
                    }
                };
            },
        );
    }
}

impl Provider for ProtocolV2Provider {
    fn start_provider(
        mut self,
        hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    ) -> Option<AbortHandle> {
        Some(
            task::spawn(async move {
                loop {
                    let updated_metadata = self.collect_plugin_metrics();
                    println!("{}: Updated metadata: {:?}", self.name, updated_metadata);
                    println!("{}: {:?}", self.name, self.state);

                    // TODO:
                    //  - detect updated metadata (use metadata_updated)
                    //  - Handle removal of outdated metrics
                    self.send_values(&hub_channel).await;

                    time::sleep(Duration::from_secs(5)).await
                }
            })
            .abort_handle(),
        )
    }
}
