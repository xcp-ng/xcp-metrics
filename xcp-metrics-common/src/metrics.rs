//! Common metrics data structures, mostly modelled after OpenMetrics.
use std::{collections::HashMap, time::SystemTime};

use serde::{Deserialize, Serialize};
use compact_str::CompactString;

/// Top level metric data structure.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct MetricSet {
    pub families: HashMap<CompactString, MetricFamily>,
}

/// A family of [Metric] sharing a [MetricType] and `unit`.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct MetricFamily {
    // Number of references to this family.
    pub reference_count: usize,
    pub metric_type: MetricType,
    pub unit: CompactString,
    pub help: CompactString,

    pub metrics: HashMap<uuid::Uuid, Metric>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
pub enum MetricType {
    #[default]
    Unknown,
    Gauge,
    Counter,
    StateSet,
    Info,
    Histogram,
    GaugeHistogram,
    Summary,
}

impl std::fmt::Display for MetricType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            MetricType::Unknown => "Unknown",
            MetricType::Gauge => "Gauge",
            MetricType::Counter => "Counter",
            MetricType::StateSet => "State Set",
            MetricType::Info => "Info",
            MetricType::Histogram => "Histogram",
            MetricType::GaugeHistogram => "Gauge Histogram",
            MetricType::Summary => "Summary",
        })
    }
}

#[derive(Clone, PartialEq, Debug, Eq, Hash, Serialize, Deserialize)]
pub struct Label {
    pub name: CompactString,
    pub value: CompactString,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub struct Metric {
    pub labels: Box<[Label]>,
    pub value: MetricValue,
}

#[derive(Clone, PartialEq, Debug, Serialize, Deserialize)]
pub enum MetricValue {
    Unknown(NumberValue),
    Gauge(NumberValue),
    Counter {
        total: NumberValue,
        created: Option<SystemTime>,
        exemplar: Option<Box<Exemplar>>,
    },
    Histogram {
        sum: NumberValue,
        count: u64,
        created: SystemTime,
        buckets: Box<[Bucket]>,
    },
    StateSet(Box<[State]>),
    Info(Box<[Label]>),
    Summary {
        sum: NumberValue,
        count: u64,
        created: SystemTime,
        quantile: Box<[Quantile]>,
    },
}

impl MetricValue {
    pub fn get_type(&self) -> MetricType {
        match self {
            Self::Unknown(_) => MetricType::Unknown,
            Self::Gauge(_) => MetricType::Gauge,
            Self::Counter { .. } => MetricType::Counter,
            Self::Histogram { .. } => MetricType::Histogram,
            Self::StateSet(_) => MetricType::StateSet,
            Self::Info(_) => MetricType::Info,
            Self::Summary { .. } => MetricType::Summary,
        }
    }
}

#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Bucket {
    pub count: u64,
    pub upper_bound: f64,
    pub exemplar: Option<Box<Exemplar>>,
}

#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Exemplar {
    pub value: f64,
    pub timestamp: Option<SystemTime>,
    pub labels: Box<[Label]>,
}

#[derive(Clone, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct State {
    pub enabled: bool,
    pub name: CompactString,
}

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
pub struct Quantile {
    pub quantile: f64,
    pub value: f64,
}

#[derive(Clone, Copy, Default, PartialEq, Debug, Serialize, Deserialize)]
pub enum NumberValue {
    Double(f64),
    Int64(i64),
    #[default]
    Undefined,
}
