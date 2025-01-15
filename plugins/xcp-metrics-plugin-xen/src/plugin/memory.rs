use std::os::unix::net::UnixStream;

use smallvec::smallvec;

use xcp_metrics_common::{
    metrics::{Metric, MetricType, MetricValue, NumberValue},
    protocol::{CreateFamily, ProtocolMessage, XcpMetricsStream},
};
use xen::{domctl::XenDomctlGetDomainInfo, hypercall::unix::UnixXenHypercall};

use super::{PluginMetricKind, XenMetric};

const PAGE_SIZE: u64 = 4096;

pub struct DomainMemory;

impl XenMetric for DomainMemory {
    fn make_families(&self, stream: &mut UnixStream) -> anyhow::Result<()> {
        stream.send_message(ProtocolMessage::CreateFamily(CreateFamily {
            help: "Memory reserved to a guest.".into(),
            name: "xen_domain_memory".into(),
            metric_type: MetricType::Gauge,
            unit: "bytes".into(),
        }))?;

        Ok(())
    }

    fn read_domain_metrics(
        &mut self,
        dominfo: XenDomctlGetDomainInfo,
        _: &UnixXenHypercall,
    ) -> smallvec::SmallVec<[(PluginMetricKind, Metric); 3]> {
        smallvec![(
            PluginMetricKind {
                family_name: "xen_domain_memory",
                submetric: None,
            },
            Metric {
                labels: vec![].into_boxed_slice(),
                value: MetricValue::Gauge(NumberValue::Int64(
                    (dominfo.tot_pages.0 * PAGE_SIZE) as i64
                )),
            },
        )]
    }
}
