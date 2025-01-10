use std::{iter, os::unix::net::UnixStream, time::Instant};

use smallvec::{smallvec, SmallVec};
use smol_str::ToSmolStr;
use xcp_metrics_common::{
    metrics::{Label, Metric, MetricType, MetricValue, NumberValue},
    protocol::{CreateFamily, ProtocolMessage, XcpMetricsStream},
};
use xen::{
    hypercall::{unix::UnixXenHypercall, XenHypercall},
    sysctl::{SysctlGetCpuInfo, SysctlPhysInfo, XenSysctlCpuinfo},
};

use super::{PluginMetricKind, XenMetric};

pub struct PCpuXenMetric {
    latest_instant: Instant,
    nr_cpus: usize,
    prev_pcpu_infos: Option<Box<[XenSysctlCpuinfo]>>,
}

impl PCpuXenMetric {
    pub fn new(hyp: &impl XenHypercall) -> Self {
        let nr_cpus = hyp
            .physinfo()
            .map(|physinfo| physinfo.max_cpu_id)
            .unwrap_or_default() as usize
            + 1;

        Self {
            latest_instant: Instant::now(),
            nr_cpus,
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
            submetric: Some(cpu_id.to_smolstr()),
        },
        Metric {
            labels: vec![Label {
                name: "cpu_id".into(),
                value: cpu_id.to_smolstr(),
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

impl XenMetric for PCpuXenMetric {
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
        hyp: &UnixXenHypercall,
    ) -> SmallVec<[(PluginMetricKind, Metric); 3]> {
        let mut new_pcpu_infos: Vec<XenSysctlCpuinfo> =
            vec![XenSysctlCpuinfo::default(); self.nr_cpus];

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
