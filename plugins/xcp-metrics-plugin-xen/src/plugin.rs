mod cpu;
mod memory;
mod vcpu;

use std::{collections::HashMap, os::unix::net::UnixStream, thread, time::Duration};

use compact_str::{CompactString, ToCompactString};
use enum_dispatch::enum_dispatch;
use smallvec::{smallvec, SmallVec};
use uuid::Uuid;

use xcp_metrics_common::{
    metrics::{Label, Metric},
    protocol::{ProtocolMessage, RemoveMetric, UpdateMetric, XcpMetricsStream},
};
use xen::{
    domctl::XenDomctlGetDomainInfo,
    hypercall::unix::UnixXenHypercall,
    sysctl::{SysctlGetDomainInfoList, SysctlPhysInfo, XenSysctlPhysInfo},
    DomId,
};

use cpu::{PCpuFreq, PCpuUsage};
use memory::DomainMemory;
use vcpu::VCpuUsage;

#[derive(Default)]
struct PluginState {
    domid_metrics: HashMap<u16, HashMap<PluginMetricKind, Uuid>>,
    host_metrics: HashMap<PluginMetricKind, Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct PluginMetricKind {
    family_name: &'static str,
    submetric: Option<CompactString>,
}

#[enum_dispatch]
pub(crate) trait XenMetric {
    fn make_families(&self, stream: &mut UnixStream) -> anyhow::Result<()>;

    fn read_host_metrics(
        &mut self,
        _physinfo: XenSysctlPhysInfo,
        _hyp: &UnixXenHypercall,
    ) -> SmallVec<[(PluginMetricKind, Metric); 3]> {
        smallvec![]
    }

    fn read_domain_metrics(
        &mut self,
        _dominfo: XenDomctlGetDomainInfo,
        // impl XenHypercall doesn't work due to enum_dispatch bug.
        _hyp: &UnixXenHypercall,
    ) -> SmallVec<[(PluginMetricKind, Metric); 3]> {
        smallvec![]
    }

    /// For cleaning up internal metric informations.
    fn clear_domain_metrics(&mut self, _domid: u16) {}
}

#[enum_dispatch(XenMetric)]
pub(crate) enum XenMetricEnum {
    Memory(DomainMemory),
    PCpu(PCpuUsage),
    VCpu(VCpuUsage),
    CpuFreq(PCpuFreq),
}

impl PluginState {
    /// Push a new metric value.
    pub fn push_domain_metric(
        &mut self,
        (domid, dom_uuid): (DomId, Uuid),
        stream: &mut UnixStream,
        (kind, mut metric): (PluginMetricKind, Metric),
    ) -> anyhow::Result<()> {
        let domain_metrics = self
            .domid_metrics
            .entry(domid.0)
            .or_insert_with(|| HashMap::new());

        let family_name = kind.family_name.to_compact_string();

        let &mut uuid = domain_metrics.entry(kind).or_insert_with(|| Uuid::new_v4());

        // Inject domain UUID label into metric.
        let mut labels = metric.labels.into_vec();
        labels.push(Label {
            name: "domain".into(),
            value: dom_uuid.as_hyphenated().to_compact_string(),
        });
        metric.labels = labels.into_boxed_slice();

        stream.send_message(ProtocolMessage::UpdateMetric(UpdateMetric {
            family_name,
            metric,
            uuid,
        }))?;

        Ok(())
    }

    pub fn push_host_metric(
        &mut self,
        stream: &mut UnixStream,
        (kind, metric): (PluginMetricKind, Metric),
    ) -> anyhow::Result<()> {
        let family_name = kind.family_name.to_compact_string();
        let &mut uuid = self
            .host_metrics
            .entry(kind)
            .or_insert_with(|| Uuid::new_v4());

        stream.send_message(ProtocolMessage::UpdateMetric(UpdateMetric {
            family_name,
            metric,
            uuid,
        }))?;

        Ok(())
    }
}

pub fn run_plugin(stream: &mut UnixStream, hyp: &UnixXenHypercall) -> anyhow::Result<()> {
    let mut state = PluginState::default();
    let metrics: &mut [XenMetricEnum] = &mut [
        DomainMemory.into(),
        VCpuUsage::new().into(),
        PCpuUsage::new().into(),
        PCpuFreq.into(),
    ];

    for xen_metric in metrics.as_ref() {
        xen_metric.make_families(stream)?;
    }

    loop {
        // Track what domains (still) exists.
        let mut found_domain = vec![0; 0];

        let physinfo = hyp
            .physinfo()
            .inspect_err(|e| tracing::error!("physinfo hypercall failure {e}"))?;

        // Get host metrics
        for metric in metrics
            .iter_mut()
            .map(|xen_metric| xen_metric.read_host_metrics(physinfo, hyp))
            .flatten()
        {
            tracing::debug!("Pushing {metric:?}");
            state.push_host_metric(stream, metric)?;
        }

        for domain in hyp.iter_domains() {
            let (domid, dom_uuid) = (domain.domain, domain.handle);
            found_domain.push(domid.0);

            for metric in metrics
                .iter_mut()
                .map(|xen_metric| xen_metric.read_domain_metrics(domain, hyp))
                .flatten()
            {
                tracing::debug!("Pushing {metric:?}");
                state.push_domain_metric((domid, dom_uuid), stream, metric)?;
            }
        }

        // For all domains that no longer exists, remove all their related metrics.
        let orphans = state
            .domid_metrics
            .keys()
            .filter(|domid| !found_domain.contains(domid))
            .cloned()
            .collect::<Vec<_>>();

        for domid in orphans {
            tracing::debug!("{domid} disappaered");

            metrics
                .iter_mut()
                .for_each(|xen_metric| xen_metric.clear_domain_metrics(domid));

            let Some(domain_metrics) = state.domid_metrics.remove(&domid) else {
                continue;
            };

            for (PluginMetricKind { family_name, .. }, uuid) in domain_metrics {
                stream.send_message(ProtocolMessage::RemoveMetric(RemoveMetric {
                    family_name: family_name.into(),
                    uuid,
                }))?;
            }
        }

        thread::sleep(Duration::from_secs(1));
    }
}
