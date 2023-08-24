//! Conversions between OpenMetrics types and xcp-metrics types.

use std::time::SystemTime;

use crate::metrics::{
    Bucket, Exemplar, Label, Metric, MetricFamily, MetricPoint, MetricSet, MetricType, MetricValue,
    NumberValue, Quantile, State,
};

use super::{
    CounterValue, GaugeValue, HistogramValue, InfoValue, StateSetValue, SummaryValue, UnknownValue,
};

// NOTE: `slice::into_vec` is used to have values instead of references for `into_iter`.

#[allow(non_snake_case)]
pub mod openmetrics {
    include!(concat!(env!("OUT_DIR"), "/openmetrics.rs"));
}

fn protobuf_ts_to_std<TS, E>(ts: prost_types::Timestamp) -> TS
where
    E: std::fmt::Debug,
    TS: TryFrom<prost_types::Timestamp, Error = E>,
{
    ts.try_into()
        .expect("Failure to convert between OpenMetrics timestamp and standard timestamp")
}

impl From<MetricType> for openmetrics::MetricType {
    fn from(value: MetricType) -> Self {
        match value {
            MetricType::Unknown => Self::Unknown,
            MetricType::Gauge => Self::Gauge,
            MetricType::Counter => Self::Counter,
            MetricType::StateSet => Self::StateSet,
            MetricType::Info => Self::Info,
            MetricType::Histogram => Self::Histogram,
            MetricType::GaugeHistogram => Self::GaugeHistogram,
            MetricType::Summary => Self::Summary,
        }
    }
}

impl From<openmetrics::MetricType> for MetricType {
    fn from(value: openmetrics::MetricType) -> Self {
        match value {
            openmetrics::MetricType::Unknown => Self::Unknown,
            openmetrics::MetricType::Gauge => Self::Gauge,
            openmetrics::MetricType::Counter => Self::Counter,
            openmetrics::MetricType::StateSet => Self::StateSet,
            openmetrics::MetricType::Info => Self::Info,
            openmetrics::MetricType::Histogram => Self::Histogram,
            openmetrics::MetricType::GaugeHistogram => Self::GaugeHistogram,
            openmetrics::MetricType::Summary => Self::Summary,
        }
    }
}

/// Convert a [Label] into a [openmetrics::Label].
impl From<Label> for openmetrics::Label {
    fn from(Label(name, value): Label) -> Self {
        Self {
            name: name.into_string(),
            value: value.into_string(),
        }
    }
}

impl From<openmetrics::Label> for Label {
    fn from(value: openmetrics::Label) -> Self {
        Self(value.name.into_boxed_str(), value.value.into_boxed_str())
    }
}

/// Generate a [impl `From<NumberValue>`] for a specified sub structure 'Value' type.
macro_rules! impl_from_number_value {
    ($value:ty) => {
        impl From<NumberValue> for $value {
            fn from(value: NumberValue) -> Self {
                match value {
                    NumberValue::Double(d) => Self::DoubleValue(d),
                    NumberValue::Int64(i) => Self::IntValue(i),
                    NumberValue::Undefined => Self::DoubleValue(f64::NAN),
                }
            }
        }
    };
}

/*
Doesn't work : https://github.com/rust-lang/rust/issues/86935

/// Generate a [impl `From<$value>` for NumberValue] for a specified sub structure 'Value' type.
macro_rules! impl_into_number_value {
    ($value:ty) => {
        impl From<$value> for NumberValue {
            fn from(value: $value) -> Self {
                match value {
                    $value::DoubleValue(d) => Self::Double(d),
                    $value::IntValue(i) => Self::Int64(i),
                }
            }
        }
    };
}
*/

impl_from_number_value!(openmetrics::gauge_value::Value);
impl From<openmetrics::gauge_value::Value> for NumberValue {
    fn from(value: openmetrics::gauge_value::Value) -> Self {
        match value {
            openmetrics::gauge_value::Value::DoubleValue(d) => Self::Double(d),
            openmetrics::gauge_value::Value::IntValue(i) => Self::Int64(i),
        }
    }
}
impl_from_number_value!(openmetrics::unknown_value::Value);
impl From<openmetrics::unknown_value::Value> for NumberValue {
    fn from(value: openmetrics::unknown_value::Value) -> Self {
        match value {
            openmetrics::unknown_value::Value::DoubleValue(d) => Self::Double(d),
            openmetrics::unknown_value::Value::IntValue(i) => Self::Int64(i),
        }
    }
}
impl_from_number_value!(openmetrics::histogram_value::Sum);
impl From<openmetrics::histogram_value::Sum> for NumberValue {
    fn from(value: openmetrics::histogram_value::Sum) -> Self {
        match value {
            openmetrics::histogram_value::Sum::DoubleValue(d) => Self::Double(d),
            openmetrics::histogram_value::Sum::IntValue(i) => Self::Int64(i),
        }
    }
}
impl_from_number_value!(openmetrics::summary_value::Sum);
impl From<openmetrics::summary_value::Sum> for NumberValue {
    fn from(value: openmetrics::summary_value::Sum) -> Self {
        match value {
            openmetrics::summary_value::Sum::DoubleValue(d) => Self::Double(d),
            openmetrics::summary_value::Sum::IntValue(i) => Self::Int64(i),
        }
    }
}

impl From<NumberValue> for openmetrics::counter_value::Total {
    fn from(value: NumberValue) -> Self {
        match value {
            NumberValue::Double(d) => Self::DoubleValue(d),
            NumberValue::Int64(i) => Self::IntValue(i.max(0) as _), // clamp to zero to prevent panicking
            NumberValue::Undefined => Self::DoubleValue(f64::NAN),
        }
    }
}

impl From<openmetrics::counter_value::Total> for NumberValue {
    fn from(value: openmetrics::counter_value::Total) -> Self {
        match value {
            openmetrics::counter_value::Total::DoubleValue(d) => Self::Double(d),
            openmetrics::counter_value::Total::IntValue(i) => Self::Int64(i as i64),
        }
    }
}

impl From<Exemplar> for openmetrics::Exemplar {
    fn from(
        Exemplar {
            value,
            timestamp,
            labels,
        }: Exemplar,
    ) -> Self {
        Self {
            value,
            label: labels.into_vec().into_iter().map(Into::into).collect(),
            timestamp: timestamp.map(Into::into),
        }
    }
}

impl From<openmetrics::Exemplar> for Exemplar {
    fn from(
        openmetrics::Exemplar {
            value,
            timestamp,
            label,
        }: openmetrics::Exemplar,
    ) -> Self {
        Self {
            labels: label.into_iter().map(Into::into).collect(),
            value,
            timestamp: timestamp.map(protobuf_ts_to_std),
        }
    }
}

impl From<Bucket> for openmetrics::histogram_value::Bucket {
    fn from(
        Bucket {
            count,
            upper_bound,
            exemplar,
        }: Bucket,
    ) -> Self {
        Self {
            count,
            exemplar: exemplar.map(|e| (*e).into()),
            upper_bound,
        }
    }
}

impl From<openmetrics::histogram_value::Bucket> for Bucket {
    fn from(
        openmetrics::histogram_value::Bucket {
            count,
            upper_bound,
            exemplar,
        }: openmetrics::histogram_value::Bucket,
    ) -> Self {
        Self {
            count,
            exemplar: exemplar.map(|e| Box::new(e.into())),
            upper_bound,
        }
    }
}

impl From<State> for openmetrics::state_set_value::State {
    fn from(State { enabled, name }: State) -> Self {
        Self {
            enabled,
            name: name.into_string(),
        }
    }
}

impl From<openmetrics::state_set_value::State> for State {
    fn from(
        openmetrics::state_set_value::State { enabled, name }: openmetrics::state_set_value::State,
    ) -> Self {
        Self {
            enabled,
            name: name.into_boxed_str(),
        }
    }
}

impl From<&Quantile> for openmetrics::summary_value::Quantile {
    fn from(&Quantile { quantile, value }: &Quantile) -> Self {
        Self { quantile, value }
    }
}

impl From<&openmetrics::summary_value::Quantile> for Quantile {
    fn from(
        &openmetrics::summary_value::Quantile { quantile, value }: &openmetrics::summary_value::Quantile,
    ) -> Self {
        Self { quantile, value }
    }
}

impl From<MetricValue> for openmetrics::metric_point::Value {
    fn from(value: MetricValue) -> Self {
        match value {
            MetricValue::Unknown(value) => Self::UnknownValue(openmetrics::UnknownValue {
                value: Some(value.into()),
            }),
            MetricValue::Gauge(value) => Self::GaugeValue(openmetrics::GaugeValue {
                value: Some(value.into()),
            }),
            MetricValue::Counter {
                total,
                created,
                exemplar,
            } => Self::CounterValue(openmetrics::CounterValue {
                created: created.map(Into::into),
                total: Some(total.into()),
                exemplar: exemplar.map(|e| (*e).into()),
            }),
            MetricValue::Histogram {
                sum,
                count,
                created,
                buckets,
            } => Self::HistogramValue(openmetrics::HistogramValue {
                count,
                sum: Some(sum.into()),
                created: Some(created.into()),
                buckets: buckets.into_vec().into_iter().map(Into::into).collect(),
            }),
            MetricValue::StateSet(states) => Self::StateSetValue(openmetrics::StateSetValue {
                states: states.into_vec().into_iter().map(Into::into).collect(),
            }),
            MetricValue::Info(info) => Self::InfoValue(openmetrics::InfoValue {
                info: info.into_vec().into_iter().map(Into::into).collect(),
            }),
            MetricValue::Summary {
                sum,
                count,
                created,
                quantile,
            } => Self::SummaryValue(openmetrics::SummaryValue {
                count,
                sum: Some(sum.into()),
                quantile: quantile.iter().map(Into::into).collect(),
                created: Some(created.into()),
            }),
        }
    }
}

impl From<openmetrics::metric_point::Value> for MetricValue {
    fn from(value: openmetrics::metric_point::Value) -> Self {
        match value {
            openmetrics::metric_point::Value::UnknownValue(UnknownValue { value }) => {
                Self::Unknown(value.map_or(NumberValue::default(), Into::into))
            }
            openmetrics::metric_point::Value::GaugeValue(GaugeValue { value }) => {
                Self::Gauge(value.map_or(NumberValue::default(), Into::into))
            }
            openmetrics::metric_point::Value::CounterValue(CounterValue {
                total,
                created,
                exemplar,
            }) => Self::Counter {
                created: created.map(protobuf_ts_to_std),
                exemplar: exemplar.map(|e| Box::new(e.into())),
                total: total.map(Into::into).unwrap_or_default(),
            },
            openmetrics::metric_point::Value::HistogramValue(HistogramValue {
                sum,
                count,
                created,
                buckets,
            }) => Self::Histogram {
                count,
                sum: sum.map_or(NumberValue::Double(0.0), Into::into),
                created: created.map_or_else(SystemTime::now, protobuf_ts_to_std),
                buckets: buckets.into_iter().map(Into::into).collect(),
            },
            openmetrics::metric_point::Value::StateSetValue(StateSetValue { states }) => {
                Self::StateSet(states.into_iter().map(Into::into).collect())
            }
            openmetrics::metric_point::Value::InfoValue(InfoValue { info }) => {
                Self::Info(info.into_iter().map(Into::into).collect())
            }
            openmetrics::metric_point::Value::SummaryValue(SummaryValue {
                sum,
                count,
                created,
                quantile,
            }) => Self::Summary {
                count,
                sum: sum.map_or(NumberValue::Double(0.0), Into::into),
                quantile: quantile.iter().map(Into::into).collect(),
                created: created.map_or_else(SystemTime::now, protobuf_ts_to_std),
            },
        }
    }
}

impl From<MetricPoint> for openmetrics::MetricPoint {
    fn from(MetricPoint { value, timestamp }: MetricPoint) -> Self {
        Self {
            value: Some(value.into()),
            timestamp: Some(timestamp.into()),
        }
    }
}

impl From<openmetrics::MetricPoint> for MetricPoint {
    fn from(openmetrics::MetricPoint { value, timestamp }: openmetrics::MetricPoint) -> Self {
        Self {
            value: value.map_or(MetricValue::Unknown(NumberValue::Undefined), Into::into),
            timestamp: timestamp.map_or_else(SystemTime::now, protobuf_ts_to_std),
        }
    }
}

/// Convert a [Metric] to a [openmetrics::Metric].
impl From<Metric> for openmetrics::Metric {
    fn from(
        Metric {
            labels,
            metrics_point,
        }: Metric,
    ) -> Self {
        Self {
            labels: labels.into_vec().into_iter().map(Into::into).collect(),
            metric_points: metrics_point
                .into_vec()
                .into_iter()
                .map(Into::into)
                .collect(),
        }
    }
}

impl From<openmetrics::Metric> for Metric {
    fn from(
        openmetrics::Metric {
            labels,
            metric_points,
        }: openmetrics::Metric,
    ) -> Self {
        Self {
            labels: labels.into_iter().map(Into::into).collect(),
            metrics_point: metric_points.into_iter().map(Into::into).collect(),
        }
    }
}

/// Convert a [MetricSet] into a [openmetrics::MetricSet].
impl From<MetricSet> for openmetrics::MetricSet {
    fn from(MetricSet { families }: MetricSet) -> Self {
        Self {
            metric_families: families
                .into_iter()
                .map(|(name, family)| openmetrics::MetricFamily {
                    name: name.into_string(),
                    help: family.help.into_string(),
                    unit: family.unit.into_string(),
                    r#type: openmetrics::MetricType::from(family.metric_type).into(),
                    metrics: family
                        .metrics
                        .into_values()
                        .map(|metric| metric.into())
                        .collect(),
                })
                .collect(),
        }
    }
}

/// NOTE: UUIDs are random
impl From<openmetrics::MetricSet> for MetricSet {
    fn from(openmetrics::MetricSet { metric_families }: openmetrics::MetricSet) -> Self {
        Self {
            families: metric_families
                .into_iter()
                .map(|family| {
                    (
                        family.name.into_boxed_str(),
                        MetricFamily {
                            help: family.help.into(),
                            unit: family.unit.into(),
                            metric_type: openmetrics::MetricType::from_i32(family.r#type)
                                .unwrap_or(openmetrics::MetricType::Unknown)
                                .into(),
                            metrics: family
                                .metrics
                                .into_iter()
                                .map(|metric| (uuid::Uuid::new_v4(), metric.into()))
                                .collect(),
                        },
                    )
                })
                .collect(),
        }
    }
}
