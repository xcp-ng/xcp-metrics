use std::{borrow::Cow, rc::Rc};

use xcp_metrics_common::rrdd::protocol_common::{DataSourceMetadata, DataSourceValue};
use xenctrl::XenControl;
use xenctrl_sys::xc_dominfo_t;

use crate::XenMetricsStatus;

use self::{
    domain::{DomainMemory, VCpuTime},
    host::LoadAvg,
};

mod domain;
mod host;
mod physical;

pub trait XenMetric {
    /// Generate metadata for this metric.
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata>;

    /// Check if this metric still exists.
    fn update(&mut self) -> bool;

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

    (metrics.into_boxed_slice(), XenMetricsStatus { dom_count })
}
