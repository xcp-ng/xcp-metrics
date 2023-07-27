use std::{borrow::Cow, mem::MaybeUninit};

use xcp_metrics_common::rrdd::protocol_common::{DataSourceMetadata, DataSourceValue};
use xenctrl::XenControl;
use xenctrl_sys::{xc_cpuinfo_t, xc_dominfo_t, xc_physinfo_t};

use crate::XenMetricsStatus;

use self::{
    domain::{DomainMemory, VCpuTime},
    host::LoadAvg,
    physical::{MemoryFree, MemoryTotal, PCpuTime},
};

mod domain;
mod host;
mod physical;

pub trait XenMetric {
    /// Generate metadata for this metric.
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata>;

    /// Update metric and check if this metric still exists.
    fn update(&mut self, shared: &XenMetricsShared, xc: &XenControl) -> bool;

    /// Get the value of this metric.
    fn get_value(&self) -> DataSourceValue;

    /// Get the name of the metric.
    fn get_name(&self) -> Cow<str>;
}

const XEN_PAGE_SIZE: usize = 4096; // 4 KiB

/// Generate a new list of XenMetric objects.
pub fn discover_xen_metrics(
    shared: &XenMetricsShared,
) -> (Box<[Box<dyn XenMetric>]>, XenMetricsStatus) {
    let mut metrics: Vec<Box<dyn XenMetric>> = vec![];

    // Add loadavg metric.
    metrics.push(Box::<LoadAvg>::default());

    shared.dominfos.iter().for_each(|dominfo| {
        let dom_uuid = uuid::Uuid::from_bytes(dominfo.handle);

        // Domain memory
        metrics.push(Box::new(DomainMemory::new(dominfo.domid, dom_uuid)));

        // vCPUs
        for vcpuid in 0..=dominfo.max_vcpu_id {
            metrics.push(Box::new(VCpuTime::new(vcpuid, dominfo.domid, dom_uuid)));
        }
    });

    // pcpu infos
    shared
        .cpuinfos
        .iter()
        .enumerate()
        .for_each(|(cpu_index, _)| metrics.push(Box::new(PCpuTime::new(cpu_index))));

    // Memory infos
    metrics.push(Box::<MemoryTotal>::default());
    metrics.push(Box::<MemoryFree>::default());

    (
        metrics.into_boxed_slice(),
        XenMetricsStatus {
            dom_count: shared.dominfos.len() as u32,
        },
    )
}

/// Shared structure with various xenctrl informations.
pub struct XenMetricsShared {
    pub physinfo: Option<xc_physinfo_t>,
    pub dominfos: Vec<xc_dominfo_t>,
    pub cpuinfos: Vec<xc_cpuinfo_t>,

    cpuinfos_buffer: Vec<MaybeUninit<xc_cpuinfo_t>>,
}

impl XenMetricsShared {
    pub fn new(xc: &XenControl) -> Self {
        let mut values = Self {
            cpuinfos: vec![],
            cpuinfos_buffer: vec![],
            dominfos: vec![],
            physinfo: None,
        };

        values.update(xc);

        values
    }

    pub fn update(&mut self, xc: &XenControl) {
        self.dominfos = (0..)
            .map_while(|i| xc.domain_getinfo(i).ok().flatten())
            .collect();

        self.physinfo = xc.physinfo().ok();

        self.cpuinfos_buffer.resize_with(
            self.physinfo
                .map(|physinfo| physinfo.nr_cpus as usize)
                .unwrap_or(0),
            MaybeUninit::zeroed,
        );

        self.cpuinfos = xc
            .get_cpuinfo(&mut self.cpuinfos_buffer)
            .map(|infos| infos.to_vec())
            .unwrap_or_default();
    }
}
