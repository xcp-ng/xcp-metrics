use std::{mem::MaybeUninit, rc::Rc};

use xcp_metrics_plugin_common::protocol_v3::utils::SimpleMetricFamily;
use xenctrl::XenControl;
use xenctrl_sys::{xc_cpuinfo_t, xc_dominfo_t, xc_physinfo_t};

use self::{
    domain::VCpuTime,
    host::LoadAvg,
    physical::{MemoryFree, MemoryTotal, PCpuTime},
};

mod domain;
mod host;
mod physical;

pub struct XenMetricsShared {
    pub physinfo: Option<xc_physinfo_t>,
    pub dominfos: Vec<xc_dominfo_t>,
    pub cpuinfos: Vec<xc_cpuinfo_t>,
    pub xc: Rc<XenControl>,

    pub cpuinfos_buffer: Vec<MaybeUninit<xc_cpuinfo_t>>,
}

impl XenMetricsShared {
    pub fn new(xc: Rc<XenControl>) -> Self {
        Self {
            xc,
            cpuinfos: vec![],
            cpuinfos_buffer: vec![],
            dominfos: vec![],
            physinfo: None,
        }
    }

    pub fn update(&mut self) {
        self.dominfos = (0..)
            .map_while(|i| self.xc.domain_getinfo(i).ok().flatten())
            .collect();

        self.physinfo = self.xc.physinfo().ok();

        self.cpuinfos_buffer.resize_with(
            self.physinfo
                .map(|physinfo| physinfo.nr_cpus as usize)
                .unwrap_or(0),
            MaybeUninit::zeroed,
        );

        self.cpuinfos = self
            .xc
            .get_cpuinfo(&mut self.cpuinfos_buffer)
            .map(|infos| infos.to_vec())
            .unwrap_or_default();
    }
}

pub trait XenMetric {
    /// Get the family name and family (with metrics).
    fn get_family(&mut self, shared: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)>;
}

const XEN_PAGE_SIZE: usize = 4096; // 4 KiB

pub fn discover_xen_metrics(_xc: Rc<XenControl>) -> Box<[Box<dyn XenMetric>]> {
    let mut metrics: Vec<Box<dyn XenMetric>> = vec![];

    // Add loadavg metric.
    metrics.push(Box::<LoadAvg>::default());

    // vCPU metrics
    metrics.push(Box::<VCpuTime>::default());

    // pcpu infos
    metrics.push(Box::<PCpuTime>::default());

    // Memory infos
    metrics.push(Box::<MemoryTotal>::default());
    metrics.push(Box::<MemoryFree>::default());

    metrics.into_boxed_slice()
}
