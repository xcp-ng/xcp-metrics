use std::{borrow::Cow, mem::MaybeUninit, rc::Rc};

use xcp_metrics_common::rrdd::protocol_common::{DataSourceMetadata, DataSourceValue};
use xenctrl::XenControl;
use xenctrl_sys::xc_dominfo_t;

use crate::{update_once::UpdateOnce, XenMetricsStatus};

use self::{
    domain::{DomainMemory, VCpuTime},
    host::LoadAvg,
    physical::{MemoryFree, MemoryTotal, PCpuTime, SharedPCpuSlice, SharedPhysInfo},
};

mod domain;
mod host;
mod physical;

pub trait XenMetric {
    /// Generate metadata for this metric.
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata>;

    /// Check if this metric still exists (token is unique per update).
    fn update(&mut self, token: uuid::Uuid) -> bool;

    /// Get the value of this metric.
    fn get_value(&self) -> DataSourceValue;

    /// Get the name of the metric.
    fn get_name(&self) -> Cow<str>;
}

const XEN_PAGE_SIZE: usize = 4096; // 4 KiB

pub fn discover_xen_metrics(xc: Rc<XenControl>) -> (Box<[Box<dyn XenMetric>]>, XenMetricsStatus) {
    let mut metrics: Vec<Box<dyn XenMetric>> = vec![];

    let mut dom_count = 0;

    // Add loadavg metric.
    metrics.push(Box::<LoadAvg>::default());

    for domid in 0.. {
        match xc.domain_getinfo(domid) {
            Ok(Some(xc_dominfo_t {
                handle,
                max_vcpu_id,
                ..
            })) => {
                // Domain exists
                dom_count += 1;

                // Domain memory
                let dom_uuid = uuid::Uuid::from_bytes(handle);

                metrics.push(Box::new(DomainMemory::new(xc.clone(), domid, dom_uuid)));

                // vCPUs
                for vcpuid in 0..=max_vcpu_id {
                    metrics.push(Box::new(VCpuTime::new(xc.clone(), vcpuid, domid, dom_uuid)));
                }
            }
            _ => break,
        }
    }

    // pcpu infos
    let physinfo = xc.physinfo();

    if let Ok(physinfo) = physinfo {
        let pcpu_slice = Rc::new(UpdateOnce::new(SharedPCpuSlice::new(
            xc.clone(),
            physinfo.nr_cpus as usize,
        )));

        let mut cpuinfos = vec![MaybeUninit::zeroed(); physinfo.nr_cpus as usize];

        if let Ok(infos) = xc.get_cpuinfo(&mut cpuinfos) {
            infos.iter().enumerate().for_each(|(cpu_index, _)| {
                metrics.push(Box::new(PCpuTime::new(cpu_index, pcpu_slice.clone())));
            })
        }
    }

    // Memory infos
    let shared_physinfo = Rc::new(UpdateOnce::new(SharedPhysInfo::new(xc.clone())));

    metrics.push(Box::new(MemoryTotal::new(shared_physinfo.clone())));
    metrics.push(Box::new(MemoryFree::new(shared_physinfo)));

    (metrics.into_boxed_slice(), XenMetricsStatus { dom_count })
}
