use std::time::Instant;

use xcp_metrics_common::metrics::{Label, MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::protocol_v3::utils::{SimpleMetric, SimpleMetricFamily};
use xenctrl_sys::xc_cpuinfo_t;

use super::{XenMetric, XenMetricsShared, XEN_PAGE_SIZE};

pub struct PCpuTime {
    latest_idle_time: Vec<Option<u64>>,
    latest_instant: Instant,
}

impl PCpuTime {
    pub fn new() -> Self {
        Self {
            latest_idle_time: vec![],
            latest_instant: Instant::now(),
        }
    }

    fn get_simple_metric(&mut self, cpuinfo: &xc_cpuinfo_t, id: usize) -> Option<SimpleMetric> {
        let latest_idle_time = self
            .latest_idle_time
            .get_mut(id)
            .expect("Vector has not been resized");

        if let Some(idle_time) = latest_idle_time {
            // Get and replace previous cpu time.
            let previous_idle_time = *idle_time;
            *idle_time = cpuinfo.idletime;
            let current_idle_time = *idle_time;

            // Update instant
            Some(SimpleMetric {
                labels: vec![Label("id".into(), id.to_string().into())],
                value: MetricValue::Gauge(NumberValue::Double(
                    // Compute busy ratio over time.
                    1.0 - (((current_idle_time - previous_idle_time) as f64)
                        / 1.0e9
                        / self.latest_instant.elapsed().as_secs_f64()),
                )),
            })
        } else {
            // We don't have the previous time.
            self.latest_idle_time[id].replace(cpuinfo.idletime);

            None
        }
    }
}

impl XenMetric for PCpuTime {
    fn get_family(&mut self, shared: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)> {
        self.latest_idle_time.resize(shared.cpuinfos.len(), None);

        let metrics = shared
            .cpuinfos
            .iter()
            .enumerate()
            .filter_map(|(id, cpuinfo)| self.get_simple_metric(cpuinfo, id))
            .collect();

        self.latest_instant = Instant::now();

        Some((
            "cpu".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "".into(),
                help: "Physical cpu usage".into(),
                metrics,
            },
        ))
    }
}

impl Default for PCpuTime {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default)]
pub struct MemoryTotal;

impl XenMetric for MemoryTotal {
    fn get_family(&mut self, shared: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)> {
        Some((
            "memory_total".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "KiB".into(),
                help: "Total amount of memory in the host".into(),
                metrics: vec![SimpleMetric {
                    labels: vec![],
                    value: MetricValue::Gauge(NumberValue::Int64(
                        shared.physinfo?.total_pages as i64,
                    )),
                }],
            },
        ))
    }
}

#[derive(Default)]
pub struct MemoryFree;

impl XenMetric for MemoryFree {
    fn get_family(&mut self, shared: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)> {
        Some((
            "memory_free".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "KiB".into(),
                help: "Total amount of free memory".into(),
                metrics: vec![SimpleMetric {
                    labels: vec![],
                    value: MetricValue::Gauge(NumberValue::Int64(
                        ((shared.physinfo?.total_pages * XEN_PAGE_SIZE as u64) / 1024) as i64,
                    )),
                }],
            },
        ))
    }
}
