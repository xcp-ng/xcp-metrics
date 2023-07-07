//! Common metrics data structures, mostly modelled after OpenMetrics.
use std::{collections::HashMap, time::SystemTime};

/// Top level metric data structure.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct MetricSet {
    pub families: HashMap<Box<str>, MetricFamily>,
}

/// A family of [Metric] sharing a [MetricType] and `unit`.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct MetricFamily {
    pub metric_type: MetricType,
    pub unit: Box<str>,
    pub help: Box<str>,

    pub metrics: HashMap<uuid::Uuid, Metric>,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Debug)]
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

#[derive(Clone, PartialEq, Debug)]
pub struct Label(
    /// Label name
    pub Box<str>,
    /// Label value
    pub Box<str>,
);

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Metric {
    pub labels: Box<[Label]>,
    pub metrics_point: Box<[MetricPoint]>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MetricPoint {
    pub value: MetricValue,
    pub timestamp: SystemTime,
}

#[derive(Clone, PartialEq, Debug)]
pub enum MetricValue {
    Unknown(NumberValue),
    Gauge(NumberValue),
    Counter {
        total: NumberValue,
        created: SystemTime,
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

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Bucket {
    pub count: u64,
    pub upper_bound: f64,
    pub exemplar: Exemplar,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct Exemplar {
    pub value: f64,
    pub timestamp: Option<SystemTime>,
    pub labels: Box<[Label]>,
}

#[derive(Clone, Default, PartialEq, Debug)]
pub struct State {
    pub enabled: bool,
    pub name: Box<str>,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub struct Quantile {
    pub quantile: f64,
    pub value: f64,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub enum NumberValue {
    Double(f64),
    Int64(i64),
    #[default]
    Undefined,
}
