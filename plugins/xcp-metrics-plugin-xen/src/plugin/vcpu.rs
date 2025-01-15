use std::{collections::HashMap, iter, os::unix::net::UnixStream, time::Instant};

use compact_str::ToCompactString;
use smallvec::{smallvec, SmallVec};

use xcp_metrics_common::{
    metrics::{Label, Metric, MetricType, MetricValue, NumberValue},
    protocol::{CreateFamily, ProtocolMessage, XcpMetricsStream},
};
use xen::{
    domctl::{DomctlGetVCpuInfo, XenDomctlGetDomainInfo, XenDomctlGetVCpuInfo},
    hypercall::unix::UnixXenHypercall,
    sysctl::XenSysctlPhysInfo,
};

use super::{PluginMetricKind, XenMetric};

pub struct VCpuUsage {
    // We need to keep track of the previous instant as latest_instant is ~now.
    previous_instant: Option<Instant>,
    latest_instant: Instant,
    prev_vcpu_infos: HashMap<u16, SmallVec<[XenDomctlGetVCpuInfo; 8]>>,
}

impl VCpuUsage {
    pub fn new() -> Self {
        Self {
            previous_instant: None,
            latest_instant: Instant::now(),
            prev_vcpu_infos: HashMap::new(),
        }
    }
}

fn generate_vcpu_usage(
    (vcpu_info, prev_vcpu_info): (&XenDomctlGetVCpuInfo, &XenDomctlGetVCpuInfo),
    latest_instant: Instant,
) -> (PluginMetricKind, Metric) {
    // xcp-rrdd: Workaround for Xen leaking the flag XEN_RUNSTATE_UPDATE; using a mask of its complement ~(1 << 63)
    // Then convert from nanoseconds to seconds
    let cputime = (vcpu_info.cpu_time.0 & !(1u64 << 63)) as f64 / 1.0e9;
    // Do the same for previous cpu time.
    let prev_cputime = (prev_vcpu_info.cpu_time.0 & !(1u64 << 63)) as f64 / 1.0e9;

    (
        PluginMetricKind {
            family_name: "xen_vcpu_time",
            submetric: Some(vcpu_info.vcpu.to_compact_string()),
        },
        Metric {
            labels: vec![Label {
                name: "vcpu_id".into(),
                value: vcpu_info.vcpu.to_compact_string(),
            }]
            .into_boxed_slice(),
            value: MetricValue::Gauge(NumberValue::Double(f64::max(
                0.0,
                (cputime - prev_cputime) / latest_instant.elapsed().as_secs_f64(),
            ))),
        },
    )
}

impl XenMetric for VCpuUsage {
    fn make_families(&self, stream: &mut UnixStream) -> anyhow::Result<()> {
        stream.send_message(ProtocolMessage::CreateFamily(CreateFamily {
            help: "Time taken running a VCPU".into(),
            name: "xen_vcpu_time".into(),
            metric_type: MetricType::Gauge,
            unit: "".into(),
        }))?;

        Ok(())
    }

    fn read_domain_metrics(
        &mut self,
        dominfo: XenDomctlGetDomainInfo,
        hyp: &UnixXenHypercall,
    ) -> SmallVec<[(PluginMetricKind, Metric); 3]> {
        let new_vcpu_infos: SmallVec<[_; 8]> = (0..=dominfo.max_vcpu_id)
            .map(|vcpu_id| {
                hyp.get_vcpu_info(dominfo.domain, vcpu_id)
                    .inspect_err(|e| tracing::error!("get_vcpu_info failure: {e}"))
                    .unwrap_or_default()
            })
            .collect();

        let metrics = if let Some((previous_vcpu_infos, previous_instant)) = self
            .prev_vcpu_infos
            .get(&dominfo.domain.0)
            .zip(self.previous_instant)
        {
            iter::zip(&new_vcpu_infos, previous_vcpu_infos)
                .map(|vcpus_infos| generate_vcpu_usage(vcpus_infos, previous_instant))
                .collect()
        } else {
            smallvec![]
        };

        self.prev_vcpu_infos
            .insert(dominfo.domain.0, new_vcpu_infos);
        metrics
    }

    fn read_host_metrics(
        &mut self,
        _physinfo: XenSysctlPhysInfo,
        _hyp: &UnixXenHypercall,
    ) -> SmallVec<[(PluginMetricKind, Metric); 3]> {
        self.previous_instant = Some(self.latest_instant);
        self.latest_instant = Instant::now();

        smallvec![]
    }

    fn clear_domain_metrics(&mut self, domid: u16) {
        self.prev_vcpu_infos.remove(&domid);
    }
}
