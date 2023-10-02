//! Simpler metrics representation for plugins.

use std::{collections::HashMap, time::SystemTime};

use xcp_metrics_common::metrics::{
    Label, Metric, MetricFamily, MetricPoint, MetricSet, MetricType, MetricValue,
};

#[derive(Clone, Debug)]
pub struct SimpleMetricSet {
    pub families: HashMap<String, SimpleMetricFamily>,
}

impl From<SimpleMetricSet> for MetricSet {
    fn from(SimpleMetricSet { families }: SimpleMetricSet) -> Self {
        Self {
            families: families
                .into_iter()
                .map(|(name, family)| (name.into_boxed_str(), family.into()))
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SimpleMetricFamily {
    pub metric_type: MetricType,
    pub unit: Box<str>,
    pub help: Box<str>,

    pub metrics: Vec<SimpleMetric>,
}

impl From<SimpleMetricFamily> for MetricFamily {
    fn from(
        SimpleMetricFamily {
            metric_type,
            unit,
            help,
            metrics,
        }: SimpleMetricFamily,
    ) -> Self {
        Self {
            metric_type,
            unit,
            help,
            metrics: metrics
                .into_iter()
                .map(|simple_metric| (uuid::Uuid::new_v4(), Metric::from(simple_metric)))
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SimpleMetric {
    pub labels: Vec<Label>,
    pub value: MetricValue,
}

impl From<SimpleMetric> for Metric {
    fn from(SimpleMetric { labels, value }: SimpleMetric) -> Self {
        Self {
            labels: labels.into_boxed_slice(),
            metrics_point: vec![MetricPoint {
                value,
                timestamp: SystemTime::now(),
            }]
            .into_boxed_slice(),
        }
    }
}
