use std::time::{Duration, SystemTime};

use crate::metrics::{
    Bucket, Exemplar, Label, MetricPoint, MetricValue, NumberValue, Quantile, State,
};

use super::convert::openmetrics;

/// Test conversions between xcp-metrics and OpenMetrics Gauge.
#[test]
fn metrics_to_openmetrics_gauge() {
    let metric_point = MetricPoint {
        value: MetricValue::Gauge(NumberValue::Int64(42)),
        timestamp: SystemTime::now(),
    };

    let om_metric_point = openmetrics::MetricPoint::from(metric_point.clone());

    let decoded_metric_point = MetricPoint::from(om_metric_point);

    assert_eq!(metric_point, decoded_metric_point);
}

/// Test conversions between xcp-metrics and OpenMetrics Counter.
#[test]
fn metrics_to_openmetrics_counter() {
    let metric_point = MetricPoint {
        value: MetricValue::Counter {
            total: NumberValue::Int64(42),
            created: SystemTime::UNIX_EPOCH + Duration::from_secs(123456789),
            exemplar: None,
        },
        timestamp: SystemTime::now(),
    };

    let om_metric_point = openmetrics::MetricPoint::from(metric_point.clone());

    let decoded_metric_point = MetricPoint::from(om_metric_point);

    assert_eq!(metric_point, decoded_metric_point);
}

/// Test conversions between xcp-metrics and OpenMetrics Info.
#[test]
fn metrics_to_openmetrics_info() {
    let metric_point = MetricPoint {
        value: MetricValue::Info(vec![Label("a".into(), "b".into())].into()),
        timestamp: SystemTime::now(),
    };

    let om_metric_point = openmetrics::MetricPoint::from(metric_point.clone());

    let decoded_metric_point = MetricPoint::from(om_metric_point);

    assert_eq!(metric_point, decoded_metric_point);
}

/// Test conversions between xcp-metrics and OpenMetrics Unknown.
#[test]
fn metrics_to_openmetrics_unknown() {
    let metric_point = MetricPoint {
        value: MetricValue::Unknown(NumberValue::Int64(123)),
        timestamp: SystemTime::now(),
    };

    let om_metric_point = openmetrics::MetricPoint::from(metric_point.clone());

    let decoded_metric_point = MetricPoint::from(om_metric_point);

    assert_eq!(metric_point, decoded_metric_point);
}

/// Test conversions between xcp-metrics and OpenMetrics StateSet.
#[test]
fn metrics_to_openmetrics_state_set() {
    let metric_point = MetricPoint {
        value: MetricValue::StateSet(
            vec![
                State {
                    enabled: true,
                    name: "A".into(),
                },
                State {
                    enabled: false,
                    name: "B".into(),
                },
            ]
            .into(),
        ),
        timestamp: SystemTime::now(),
    };

    let om_metric_point = openmetrics::MetricPoint::from(metric_point.clone());

    let decoded_metric_point = MetricPoint::from(om_metric_point);

    assert_eq!(metric_point, decoded_metric_point);
}

/// Test conversions between xcp-metrics and OpenMetrics Summary.
#[test]
fn metrics_to_openmetrics_summary() {
    let metric_point = MetricPoint {
        value: MetricValue::Summary {
            sum: NumberValue::Int64(1),
            count: 2,
            created: SystemTime::UNIX_EPOCH + Duration::from_secs(123456789),
            quantile: vec![
                Quantile {
                    quantile: 1.0,
                    value: 1.0,
                },
                Quantile {
                    quantile: 2.0,
                    value: 2.0,
                },
            ]
            .into(),
        },
        timestamp: SystemTime::now(),
    };

    let om_metric_point = openmetrics::MetricPoint::from(metric_point.clone());

    let decoded_metric_point = MetricPoint::from(om_metric_point);

    assert_eq!(metric_point, decoded_metric_point);
}

/// Test conversions between xcp-metrics and OpenMetrics Histogram.
#[test]
fn metrics_to_openmetrics_histogram() {
    let exemplar = Some(
        Exemplar {
            value: 1.0,
            timestamp: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(123456789)),
            labels: vec![Label("a".into(), "b".into())].into(),
        }
        .into(),
    );

    let metric_point = MetricPoint {
        value: MetricValue::Histogram {
            sum: NumberValue::Int64(1),
            count: 1,
            created: SystemTime::UNIX_EPOCH + Duration::from_secs(123456789),
            buckets: vec![
                Bucket {
                    count: 1,
                    upper_bound: 1.0,
                    exemplar: None,
                },
                Bucket {
                    count: 1,
                    upper_bound: 2.0,
                    exemplar,
                },
            ]
            .into(),
        },
        timestamp: SystemTime::now(),
    };

    let om_metric_point = openmetrics::MetricPoint::from(metric_point.clone());

    let decoded_metric_point = MetricPoint::from(om_metric_point);

    assert_eq!(metric_point, decoded_metric_point);
}
