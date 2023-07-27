use std::time::SystemTime;

use crate::metrics::{MetricPoint, MetricValue};

use super::convert::openmetrics;

/// Test conversions between xcp-metrics and OpenMetrics Gauge.
#[test]
fn metrics_to_openmetrics_gauge() {
    let metric_point = MetricPoint {
        value: MetricValue::Gauge(crate::metrics::NumberValue::Int64(42)),
        timestamp: SystemTime::now(),
    };

    let om_metric_point = openmetrics::MetricPoint::from(metric_point.clone());

    let decoded_metric_point = MetricPoint::from(om_metric_point);

    assert_eq!(metric_point, decoded_metric_point);
}
