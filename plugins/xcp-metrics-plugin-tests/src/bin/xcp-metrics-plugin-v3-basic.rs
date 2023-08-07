use std::time::Duration;
use tokio::time;

use xcp_metrics_common::metrics::{MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::protocol_v3::{
    utils::{SimpleMetric, SimpleMetricFamily, SimpleMetricSet},
    MetricsPlugin,
};

#[tokio::main]
async fn main() {
    let metrics = SimpleMetricSet {
        families: [
            (
                "life".to_string(),
                SimpleMetricFamily {
                    unit: "everything".into(),
                    help: "Answer to the Ultimate Question of Life, The Universe, and Everything"
                        .into(),
                    metric_type: MetricType::Gauge,
                    metrics: vec![SimpleMetric {
                        labels: vec![],
                        value: MetricValue::Gauge(NumberValue::Int64(42)),
                    }],
                },
            ),
            (
                "pi".to_string(),
                SimpleMetricFamily {
                    unit: "rad".into(),
                    help: "The PI number".into(),
                    metric_type: MetricType::Gauge,
                    metrics: vec![SimpleMetric {
                        labels: vec![],
                        value: MetricValue::Gauge(NumberValue::Double(std::f64::consts::PI)),
                    }],
                },
            ),
        ]
        .into_iter()
        .collect(),
    };

    let plugin = MetricsPlugin::new("xcp-metrics-plugin-basic", metrics.clone().into(), None)
        .await
        .unwrap();

    // Expose protocol v2
    loop {
        // Update sources
        plugin.update(metrics.clone().into()).await.unwrap();
        time::sleep(Duration::from_secs(1)).await;
    }
}
