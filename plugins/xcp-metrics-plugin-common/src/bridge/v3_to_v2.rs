//! Adapter to convert from protocol v3 to protocol v2.
use std::{fmt::Write, iter};

use xcp_metrics_common::{
    metrics::{
        Label, Metric, MetricFamily, MetricPoint, MetricSet, MetricType, MetricValue, NumberValue,
    },
    rrdd::{
        protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
        protocol_v2::{indexmap::IndexMap, RrddMetadata},
    },
    utils::delta::MetricSetModel,
};

/// Adapter to convert protocol v3 metrics set into protocol v2 metadata and data.
pub struct BridgeToV2 {
    model: MetricSetModel,
    latest_set: MetricSet,

    metadata: RrddMetadata,
    metadata_map: Vec<(Box<str>, Box<[Label]>)>,
}

/// Convert a Metric into a DataSourceMetadata entry.
fn metric_to_v2_metadata(
    family_name: &str,
    family: &MetricFamily,
    metric: &Metric,
) -> (Box<str>, DataSourceMetadata) {
    let owner = metric
        .labels
        .iter()
        .filter(|l| l.0.as_ref() == "owner")
        .map(|l| l.1.as_ref())
        .next()
        .unwrap_or("host");

    // Parse owner
    let owner = DataSourceOwner::try_from(owner).unwrap_or(DataSourceOwner::Host);

    let name = metric
        .labels
        .iter()
        // Ignore owner label
        .filter(|l| l.0.as_ref() != "owner")
        .fold(String::from(family_name), |mut buffer, label| {
            write!(buffer, "_{}", label.1).ok();
            buffer
        });

    let ds_type = match family.metric_type {
        MetricType::Gauge => DataSourceType::Gauge,
        MetricType::Counter => DataSourceType::Absolute,
        _ => unreachable!("MetricType should be filtered out"),
    };

    let first_metric = metric
        .metrics_point
        .first()
        .map(|metric_point| &metric_point.value);

    let value = first_metric.map_or(DataSourceValue::Undefined, |metric| match metric {
        MetricValue::Gauge(value) | MetricValue::Counter { total: value, .. } => match value {
            NumberValue::Double(_) => DataSourceValue::Float(0.0),
            NumberValue::Int64(_) => DataSourceValue::Int64(0),
            NumberValue::Undefined => DataSourceValue::Undefined,
        },
        _ => DataSourceValue::Undefined,
    });

    (
        name.into_boxed_str(),
        DataSourceMetadata {
            description: family.help.clone(),
            units: family.unit.clone(),
            ds_type,
            value,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
            owner,
            default: true,
        },
    )
}

/// Convert a MetricPoint into a protocol-v2 value.
fn metric_point_to_v2(metric_point: &MetricPoint) -> DataSourceValue {
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
            metadata: RrddMetadata {
                datasources: IndexMap::default(),
            },
            metadata_map: vec![],
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
    ) -> Option<&'a MetricPoint> {
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
            // Only consider gauge and counter values.
            .filter(|((_, family), _)| {
                matches!(family.metric_type, MetricType::Gauge | MetricType::Counter)
            })
            // Convert this data to protocol v2 metadata and mapping information.
            .map(|((family_name, family), (_, metric))| {
                (
                    metric_to_v2_metadata(family_name, family, metric),
                    (family_name.clone(), metric.labels.clone()),
                )
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
