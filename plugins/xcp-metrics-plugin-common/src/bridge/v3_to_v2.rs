//! Adapter to convert from protocol v3 to protocol v2.
use std::{collections::HashMap, iter};

use xcp_metrics_common::{
    metrics::{Label, Metric, MetricFamily, MetricValue, MetricSet, MetricValue},
    rrdd::{
        protocol_common::{DataSourceMetadata, DataSourceValue},
        protocol_v2::{indexmap::IndexMap, RrddMetadata},
    },
    utils::{
        delta::MetricSetModel,
        mapping::{CustomMapping, DefaultMapping, MetadataMapping},
    },
};

/// Adapter to convert protocol v3 metrics set into protocol v2 metadata and data.
#[derive(Clone)]
pub struct BridgeToV2 {
    model: MetricSetModel,
    latest_set: MetricSet,
    custom_mappings: HashMap<Box<str>, CustomMapping>,

    metadata: RrddMetadata,
    metadata_map: Vec<(Box<str>, Box<[Label]>)>,
}

/// Convert a MetricPoint into a protocol-v2 value.
fn metric_point_to_v2(metric_point: &MetricValue) -> DataSourceValue {
    match metric_point.value {
        MetricValue::Gauge(value) => DataSourceValue::from(value),
        MetricValue::Counter { total, .. } => DataSourceValue::from(total),
        _ => DataSourceValue::Undefined,
    }
}

impl BridgeToV2 {
    pub fn new() -> Self {
        Self {
            model: MetricSetModel::default(),
            latest_set: MetricSet::default(),
            custom_mappings: HashMap::default(),
            metadata: RrddMetadata {
                datasources: IndexMap::default(),
            },
            metadata_map: vec![],
        }
    }

    pub fn with_mappings(custom_mappings: HashMap<Box<str>, CustomMapping>) -> Self {
        Self {
            custom_mappings,
            ..Default::default()
        }
    }

    fn metric_to_v2_metadata(
        &self,
        family_name: &str,
        family: &MetricFamily,
        metric: &Metric,
    ) -> Option<(Box<str>, DataSourceMetadata)> {
        if let Some(custom_mapping) = self.custom_mappings.get(family_name) {
            custom_mapping.convert(family_name, family, metric)
        } else {
            DefaultMapping.convert(family_name, family, metric)
        }
    }

    /// Update bridge information, returns true on metadata change.
    pub fn update(&mut self, metrics_set: MetricSet) -> bool {
        let delta = self.model.compute_delta(&metrics_set);
        self.model.apply_delta(&delta);

        if !delta.added_families.is_empty()
            || !delta.added_metrics.is_empty()
            || !delta.removed_metrics.is_empty()
        {
            self.latest_set = metrics_set;
            self.reset_metadata();
            true
        } else {
            self.latest_set = metrics_set;
            false
        }
    }

    pub fn get_data(&self) -> Box<[DataSourceValue]> {
        self.metadata_map
            .iter()
            .filter_map(|(family_name, labels)| self.get_first_metric_point(family_name, labels))
            .map(metric_point_to_v2)
            .collect::<Box<[DataSourceValue]>>()
    }

    pub fn get_metadata(&self) -> &RrddMetadata {
        &self.metadata
    }

    fn get_first_metric_point<'a>(
        &'a self,
        family_name: &str,
        labels: &[Label],
    ) -> Option<&'a MetricValue> {
        self.latest_set
            .families
            .get(family_name)
            .and_then(|family| {
                family
                    .metrics
                    .iter()
                    // Filter by labels
                    .filter(|(_, metric)| metric.labels.as_ref() == labels)
                    // Only take first metrics_point
                    .find_map(|(_, metric)| metric.metrics_point.first())
            })
    }

    fn reset_metadata(&mut self) {
        let (datasources, metadata_map) = self
            .latest_set
            .families
            .iter()
            // Combine family with family name and metrics.
            .flat_map(|(name, family)| {
                iter::zip(iter::repeat((name, family)), family.metrics.iter())
            })
            // Convert this data to protocol v2 metadata and mapping information.
            .filter_map(|((family_name, family), (_, metric))| {
                self.metric_to_v2_metadata(family_name, family, metric)
                    .map(|mapping| (mapping, (family_name.clone(), metric.labels.clone())))
            })
            .unzip();

        self.metadata = RrddMetadata { datasources };
        self.metadata_map = metadata_map;
    }
}

impl Default for BridgeToV2 {
    fn default() -> Self {
        Self::new()
    }
}
