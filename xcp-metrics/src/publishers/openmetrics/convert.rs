//! Conversions between OpenMetrics types and xcp-metrics types.
use xcp_metrics_common::metrics::{
    Bucket, Exemplar, Label, Metric, MetricPoint, MetricSet, MetricType, MetricValue, NumberValue,
    Quantile, State,
};

// NOTE: `slice::into_vec` is used to have values instead of references for `into_iter`.

mod openmetrics {
    include!(concat!(env!("OUT_DIR"), "/openmetrics.rs"));
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

/// Serialize [Label] into a [openmetrics::Label].
impl From<Label> for openmetrics::Label {
    fn from(Label(name, value): Label) -> Self {
        Self {
            name: name.into_string(),
            value: value.into_string(),
        }
    }
}

/// Generate a impl From<NumberValue> for a specified sub structure 'Value' type.
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

impl_from_number_value!(openmetrics::gauge_value::Value);
impl_from_number_value!(openmetrics::unknown_value::Value);
impl_from_number_value!(openmetrics::histogram_value::Sum);
impl_from_number_value!(openmetrics::summary_value::Sum);

impl From<NumberValue> for openmetrics::counter_value::Total {
    fn from(value: NumberValue) -> Self {
        match value {
            NumberValue::Double(d) => Self::DoubleValue(d),
            NumberValue::Int64(i) => Self::IntValue(i.max(0) as _), // clamp to zero to prevent panicking
            NumberValue::Undefined => Self::DoubleValue(f64::NAN),
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
            exemplar: Some(exemplar.into()),
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

impl From<&Quantile> for openmetrics::summary_value::Quantile {
    fn from(&Quantile { quantile, value }: &Quantile) -> Self {
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
                created: Some(created.into()),
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
                quantile: quantile.into_iter().map(Into::into).collect(),
                created: Some(created.into()),
            }),
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

/// Serialize [Metric] into a [openmetrics::Metric].
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

/// Serialize [MetricSet] into a [openmetrics::MetricSet].
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
