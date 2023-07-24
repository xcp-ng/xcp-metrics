use xcp_metrics_common::metrics::{MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::protocol_v3::utils::{SimpleMetric, SimpleMetricFamily};

use super::XenMetricsShared;
use crate::XenMetric;

#[derive(Default)]
pub struct LoadAvg(f64);

impl XenMetric for LoadAvg {
    fn get_family(&mut self, _: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)> {
        let proc_loadavg =
            std::fs::read_to_string("/proc/loadavg").expect("Unable to read /proc/loadavg");

        let loadavg = proc_loadavg
            .split_once(' ')
            .expect("No first element in /proc/loadavg ?")
            .0
            .parse()
            .expect("First part of /proc/loadavg not a number ?");

        Some((
            "loadavg".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "".into(),
                help: "Domain0 loadavg".into(),
                metrics: [SimpleMetric {
                    labels: vec![],
                    value: MetricValue::Gauge(NumberValue::Double(loadavg)),
                }]
                .into(),
            },
        ))
    }
}
