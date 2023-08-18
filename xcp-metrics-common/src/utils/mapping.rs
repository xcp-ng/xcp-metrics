//! xcp-metrics to protocol v2 mapping utilities
use std::fmt::Write;

use serde::{Deserialize, Serialize};

use crate::{
    metrics::{Label, Metric, MetricFamily, MetricType, MetricValue, NumberValue},
    rrdd::protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
};

pub trait MetadataMapping {
    fn convert(
        &self,
        family_name: &str,
        family: &MetricFamily,
        metric: &Metric,
    ) -> Option<(Box<str>, DataSourceMetadata)>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultMapping;

impl MetadataMapping for DefaultMapping {
    /// Convert a Metric into a DataSourceMetadata entry.
    fn convert(
        &self,
        family_name: &str,
        family: &MetricFamily,
        metric: &Metric,
    ) -> Option<(Box<str>, DataSourceMetadata)> {
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
            _ => return None, /* Non-supported */
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

        Some((
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
        ))
    }
}

/// Simple yet effective that has specified min/max/default values, and use a pattern inspired of Rust formatting for metric name.
///
/// # Pattern syntax
///
/// A pattern is a string where all occurences of `{label}` (where `label` is the
/// name of a label that exists in the metric) is replaced by the label value.
///
/// If the label doesn't exist in the metric, the occurence is not substituted and kept as is.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CustomMapping {
    pub pattern: Box<str>,
    pub min: f32,
    pub max: f32,
    pub default: bool,
}

impl CustomMapping {
    fn apply_pattern(&self, metric: &Metric) -> Box<str> {
        let mut current = self.pattern.to_string();

        // Replace all occurences of `{label}` with its value.
        metric.labels.iter().for_each(|Label(name, value)| {
            current = current.replace(format!("{{{name}}}").as_str(), value);
        });

        current.into_boxed_str()
    }
}

impl MetadataMapping for CustomMapping {
    /// Convert a Metric into a DataSourceMetadata entry.
    fn convert(
        &self,
        _: &str,
        family: &MetricFamily,
        metric: &Metric,
    ) -> Option<(Box<str>, DataSourceMetadata)> {
        let owner = metric
            .labels
            .iter()
            .filter(|l| l.0.as_ref() == "owner")
            .map(|l| l.1.as_ref())
            .next()
            .unwrap_or("host");

        // Parse owner
        let owner = DataSourceOwner::try_from(owner).unwrap_or(DataSourceOwner::Host);

        let name = self.apply_pattern(metric);

        let ds_type = match family.metric_type {
            MetricType::Gauge => DataSourceType::Gauge,
            MetricType::Counter => DataSourceType::Absolute,
            _ => return None, /* Non-supported */
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

        Some((
            name,
            DataSourceMetadata {
                description: family.help.clone(),
                units: family.unit.clone(),
                ds_type,
                value,
                min: self.min,
                max: self.max,
                owner,
                default: self.default,
            },
        ))
    }
}
