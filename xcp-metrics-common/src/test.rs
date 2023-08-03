//! Protocol v3 tests

use std::time::SystemTime;

use crate::{
    metrics::{Metric, MetricFamily, MetricPoint, MetricSet, MetricType, MetricValue, NumberValue},
    protocol_v3,
};

#[test]
fn test_protocol_v3_header() {
    let metrics_set = MetricSet {
        families: [(
            "test".into(),
            MetricFamily {
                metric_type: MetricType::Gauge,
                unit: "unit".into(),
                help: "help".into(),
                metrics: [(
                    uuid::Uuid::new_v4(),
                    Metric {
                        labels: vec![].into(),
                        metrics_point: vec![MetricPoint {
                            value: MetricValue::Gauge(NumberValue::Int64(1)),
                            timestamp: SystemTime::now(),
                        }]
                        .into(),
                    },
                )]
                .into_iter()
                .collect(),
            },
        )]
        .into_iter()
        .collect(),
    };

    // Generate raw payload.
    let mut buffer = vec![];
    protocol_v3::generate_v3(&mut buffer, None, metrics_set.clone()).unwrap();

    let (_, metrics_readed) = protocol_v3::parse_v3(&mut buffer.as_slice()).unwrap();

    // We can't lazily compare them as xcp-metrics metrics has some additional informations
    // (like internal uuid) that are randomly generated when parsing from OpenMetrics.

    // TODO: Complete this part
}
