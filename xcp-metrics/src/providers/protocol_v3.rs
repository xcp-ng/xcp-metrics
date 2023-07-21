//! Protocol v3 plugin metrics provider

use std::{
    collections::{HashMap, HashSet},
    iter,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

use tokio::{fs::File, sync::mpsc, task::JoinHandle, time};
use xcp_metrics_common::{
    metrics::{Label, Metric, MetricSet},
    protocol_v3,
};

use crate::hub::{HubPushMessage, UnregisterMetrics};

use super::Provider;

const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

// TODO: Consider supporting crazy plugins that changes metric families ?

// Summary of the changes
#[derive(Debug, Default)]
struct MetricSetDelta {
    /// New added families (name only)
    added_families: Vec<Box<str>>,

    /// Changed family metadata
    // changed_families: Vec<&'a str>,

    /// Metrics that no longer contain a family.
    /// In case they reappears, they will need to be registered again.
    orphaned_families: Vec<Box<str>>,

    // Added metrics
    added_metrics: Vec<(Box<str>, Metric)>,

    // Removed metrics
    removed_metrics: Vec<uuid::Uuid>,
    // Updated metrics
    // Currently consider updating all metrics.
    // TODO: Do some testing/benchmark about this. Maybe force-update
    //       all metrics other than Drop-heavy structures like Info,
    //       StateSet, Summary, Histogram, ...
    //
    // updated: Vec<UpdateMetrics>,
}

#[derive(Debug, Clone)]
pub struct ProtocolV3Provider {
    name: Box<str>,
    path: PathBuf,
    last_timestamp: SystemTime,

    /// Track metrics per family and labels set.
    metrics_map: HashMap<(Box<str>, Box<[Label]>), uuid::Uuid>,
    families: HashSet<Box<str>>,
}

impl ProtocolV3Provider {
    pub fn new(plugin_name: &str) -> Self {
        Self {
            name: plugin_name.into(),
            path: Path::new(METRICS_SHM_PATH).join(plugin_name),
            last_timestamp: SystemTime::now(),
            metrics_map: HashMap::new(),
            families: HashSet::new(),
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

    /// Compute variation between metrics_set and current model.
    fn compute_delta(&self, mut metrics_set: MetricSet) -> MetricSetDelta {
        // Check for new families.
        let added_families = metrics_set
            .families
            .keys()
            .filter(|name| self.families.contains(*name))
            .cloned()
            .collect();

        // Check for removed metrics.
        let removed_metrics = self
            .metrics_map
            .iter()
            .filter_map(|((name, labels), uuid)| {
                let Some(family) = metrics_set.families.get(name) else {
                    // Related family doesn't exist anymore, so do metric.
                    return Some(*uuid)
                };

                // Check for metric existence in family.
                // NOTE: As UUID is random due to conversion between raw OpenMetrics and xcp-metrics
                //       structure, we can't rely on it, and must use labels check existence.
                if !family
                    .metrics
                    .iter()
                    .any(|(_, metric)| labels == &metric.labels)
                {
                    Some(*uuid)
                } else {
                    None
                }
            })
            .collect();

        // Check for added metrics.
        let added_metrics = metrics_set
            .families
            .iter_mut()
            // Combine family name with each family metric.
            .map(|(name, family)| iter::zip(iter::repeat(name), family.metrics.iter()))
            .flatten()
            // Only consider metrics we don't have, and strip uuid.
            .filter_map(|(name, (_, metric))| {
                // Due to contains_key expecting a tuple, we need to provide it a proper tuple (by cloning).
                // TODO: Find a better solution than cloning.
                if !self
                    .metrics_map
                    .contains_key(&(name.clone(), metric.labels.clone()))
                {
                    // We don't have the metric.
                    Some((name.clone(), metric.clone()))
                } else {
                    None
                }
            })
            .collect();

        // Check for families that doesn't exist anymore.
        let orphaned_families = self
            .families
            .iter()
            .filter(|family| metrics_set.families.contains_key(*family))
            .cloned()
            .collect();

        MetricSetDelta {
            added_families,
            orphaned_families,
            added_metrics,
            removed_metrics,
        }
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
                        let MetricSetDelta {
                            added_families,
                            orphaned_families,
                            removed_metrics,
                            added_metrics,
                        } = self.compute_delta(new_metrics);

                        // Remove metrics
                        removed_metrics.iter().for_each(|&uuid| {
                            if let Err(e) = hub_channel.send(HubPushMessage::UnregisterMetrics(
                                UnregisterMetrics { uuid },
                            )) {
                                tracing::error!("Unregister error {e}");
                            }
                        });

                        // Update mapping, only keep those non-removed
                        self.metrics_map
                            .retain(|_, uuid| !removed_metrics.contains(uuid));

                        // Remove orphaned families.
                        self.families
                            .retain(|name| !orphaned_families.contains(name));

                        // Add new metrics
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
