use std::{iter, os::unix::net::UnixStream, time::Instant};

use compact_str::ToCompactString;
use smallvec::{smallvec, SmallVec};

use xcp_metrics_common::{
    metrics::{Label, Metric, MetricType, MetricValue, NumberValue},
    protocol::{CreateFamily, ProtocolMessage, XcpMetricsStream},
};
use xen::{
    hypercall::unix::UnixXenHypercall,
    sysctl::{SysctlGetCpuInfo, SysctlGetPmOp, XenSysctlCpuinfo, XenSysctlPhysInfo},
};

use super::{PluginMetricKind, XenMetric};

// TODO: use a passed physinfo
pub struct PCpuUsage {
    latest_instant: Instant,
    prev_pcpu_infos: Option<Box<[XenSysctlCpuinfo]>>,
}

impl PCpuUsage {
    pub fn new() -> Self {
        Self {
            latest_instant: Instant::now(),
            prev_pcpu_infos: None,
        }
    }
}

fn generate_pcpu_usage(
    cpu_id: usize,
    (pcpu_info, prev_pcpu_info): (&XenSysctlCpuinfo, &XenSysctlCpuinfo),
    latest_instant: Instant,
) -> (PluginMetricKind, Metric) {
    (
        PluginMetricKind {
            family_name: "xen_cpu_time",
            submetric: Some(cpu_id.to_compact_string()),
        },
        Metric {
            labels: vec![Label {
                name: "cpu_id".into(),
                value: cpu_id.to_compact_string(),
            }]
            .into_boxed_slice(),
            value: MetricValue::Gauge(NumberValue::Double(f64::max(
                0.0,
                1.0 - f64::max(
                    0.0,
                    ((pcpu_info.idletime.0 - prev_pcpu_info.idletime.0) as f64)
                        / 1.0e9
                        / latest_instant.elapsed().as_secs_f64(),
                ),
            ))),
        },
    )
}

impl XenMetric for PCpuUsage {
    fn make_families(&self, stream: &mut UnixStream) -> anyhow::Result<()> {
        stream.send_message(ProtocolMessage::CreateFamily(CreateFamily {
            help: "Time taken running a CPU core".into(),
            name: "xen_cpu_time".into(),
            metric_type: MetricType::Gauge,
            unit: "".into(),
        }))?;

        Ok(())
    }

    fn read_host_metrics(
        &mut self,
        physinfo: XenSysctlPhysInfo,
        hyp: &UnixXenHypercall,
    ) -> SmallVec<[(PluginMetricKind, Metric); 3]> {
        let mut new_pcpu_infos: Vec<XenSysctlCpuinfo> =
            vec![XenSysctlCpuinfo::default(); (physinfo.max_cpu_id + 1) as _];

        match hyp.get_cpu_info(&mut new_pcpu_infos) {
            Ok(count) => new_pcpu_infos.truncate(count),
            Err(e) => {
                tracing::error!("get_cpu_info failure: {e}");
                return smallvec![];
            }
        }

        let metrics = if let Some(previous_pcpu_infos) = &self.prev_pcpu_infos {
            iter::zip(&new_pcpu_infos, previous_pcpu_infos)
                .enumerate()
                .map(|(cpu_id, pcpus_info)| {
                    generate_pcpu_usage(cpu_id, pcpus_info, self.latest_instant)
                })
                .collect()
        } else {
            smallvec![]
        };

        self.prev_pcpu_infos = Some(new_pcpu_infos.into_boxed_slice());
        self.latest_instant = Instant::now();
        metrics
    }
}

pub struct PCpuFreq;

impl XenMetric for PCpuFreq {
    fn make_families(&self, stream: &mut UnixStream) -> anyhow::Result<()> {
        stream.send_message(ProtocolMessage::CreateFamily(CreateFamily {
            help: "Average frequency of a CPU core".into(),
            name: "xen_cpu_freq".into(),
            metric_type: MetricType::Gauge,
            unit: "hz".into(),
        }))?;

        Ok(())
    }

    fn read_host_metrics(
        &mut self,
        physinfo: XenSysctlPhysInfo,
        hyp: &UnixXenHypercall,
    ) -> SmallVec<[(PluginMetricKind, Metric); 3]> {
        (0..=physinfo.max_cpu_id)
            .filter_map(|cpuid| {
                // Ignore all failing reads.
                hyp.get_cpufreq_avgfreq(cpuid)
                    .inspect_err(|e| {
                        tracing::warn!("get_cpufreq_avg failure for cpuid:{cpuid}: {e}")
                    })
                    .ok()
                    .map(|freq| (cpuid, freq))
            })
            .map(|(cpuid, freq)| {
                (
                    PluginMetricKind {
                        family_name: "xen_cpu_freq",
                        submetric: Some(cpuid.to_compact_string()),
                    },
                    Metric {
                        labels: vec![Label {
                            name: "cpu_id".into(),
                            value: cpuid.to_compact_string(),
                        }]
                        .into_boxed_slice(),
                        value: MetricValue::Gauge(NumberValue::Int64(freq as i64)),
                    },
                )
            })
            .collect()
    }
}
