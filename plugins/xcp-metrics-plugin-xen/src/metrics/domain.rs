use std::{collections::HashMap, time::Instant};

use uuid::Uuid;
use xcp_metrics_common::metrics::{Label, MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::protocol_v3::utils::{SimpleMetric, SimpleMetricFamily};
use xenctrl::XenControl;
use xenctrl_sys::xc_dominfo_t;

use crate::XenMetric;

use super::{XenMetricsShared, XEN_PAGE_SIZE};

pub struct VCpuTime {
    latest_time: HashMap<(u32 /* domid */, u32 /* vcpu_id */), u64>,
    latest_instant: Instant,
}

impl VCpuTime {
    pub fn new() -> Self {
        Self {
            latest_instant: Instant::now(),
            latest_time: HashMap::new(),
        }
    }

    fn get_simple_metric(
        &mut self,
        xc: &XenControl,
        dominfo: &xc_dominfo_t,
        vcpu_id: u32,
    ) -> Option<SimpleMetric> {
        let vcpu_info = xc.vcpu_getinfo(dominfo.domid, vcpu_id).ok()?;
        let latest_time = self.latest_time.get_mut(&(dominfo.domid, vcpu_id));

        if let Some(time) = latest_time {
            // Get and replace previous cpu time.
            let previous_time = *time;
            *time = vcpu_info.cpu_time;
            let current_time = *time;

            // xcp-rrdd: Workaround for Xen leaking the flag XEN_RUNSTATE_UPDATE; using a mask of its complement ~(1 << 63)
            // Then convert from nanoseconds to seconds
            let cputime = (current_time & !(1u64 << 63)) as f64 / 1.0e9;
            // Do the same for previous cpu time.
            let previous_cputime = (previous_time & !(1u64 << 63)) as f64 / 1.0e9;

            Some(SimpleMetric {
                labels: vec![
                    Label("id".into(), format!("{vcpu_id}").into()),
                    Label(
                        "owner".into(),
                        format!("vm {}", Uuid::from_bytes(dominfo.handle).as_hyphenated()).into(),
                    ),
                ],
                value: MetricValue::Gauge(NumberValue::Double(f64::max(
                    0.0,
                    (cputime - previous_cputime) / self.latest_instant.elapsed().as_secs_f64(),
                ))),
            })
        } else {
            // We don't have the previous time.
            self.latest_time
                .insert((dominfo.domid, vcpu_id), vcpu_info.cpu_time);

            None
        }
    }
}

impl Default for VCpuTime {
    fn default() -> Self {
        Self::new()
    }
}

impl XenMetric for VCpuTime {
    fn get_family(&mut self, shared: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)> {
        let metrics = shared
            .dominfos
            .iter()
            .flat_map(|dominfo| {
                (0..=dominfo.max_vcpu_id)
                    .filter_map(|vcpu_id| self.get_simple_metric(&shared.xc, dominfo, vcpu_id))
                    .collect::<Vec<_>>()
            })
            .collect();

        self.latest_instant = Instant::now();

        Some((
            "vcpu".into(),
            SimpleMetricFamily {
                help: "vCPU usages".into(),
                unit: "usage".into(),
                metric_type: MetricType::Gauge,
                metrics,
            },
        ))
    }
}

#[derive(Clone, Default)]
pub struct DomainMemory;

impl XenMetric for DomainMemory {
    fn get_family(&mut self, shared: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)> {
        let metrics = shared
            .dominfos
            .iter()
            .map(|dominfo| {
                let bytes_used = dominfo.nr_pages * XEN_PAGE_SIZE as u64;
                let uuid = Uuid::from_bytes(dominfo.handle);

                SimpleMetric {
                    labels: vec![Label(
                        "owner".into(),
                        format!("vm {}", uuid.as_hyphenated()).into(),
                    )],
                    value: MetricValue::Gauge(NumberValue::Int64(bytes_used as i64)),
                }
            })
            .collect();

        Some((
            "memory".into(),
            SimpleMetricFamily {
                help: "Memory currently allocated to VM".into(),
                metric_type: MetricType::Gauge,
                unit: "bytes".into(),
                metrics,
            },
        ))
    }
}
