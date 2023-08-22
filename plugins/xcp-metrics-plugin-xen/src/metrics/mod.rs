use std::{mem::MaybeUninit, rc::Rc};

use xcp_metrics_plugin_common::protocol_v3::utils::SimpleMetricFamily;
use xenctrl::{CxStat, PxStat, XenControl};
use xenctrl_sys::{xc_cpuinfo_t, xc_dominfo_t, xc_physinfo_t};

use self::{
    domain::{DomainMemory, VCpuTime},
    host::LoadAvg,
    physical::{MemoryFree, MemoryTotal, PCpuAvg, PCpuTime},
    pm::{CpuAvgFrequency, CpuCState, CpuPState},
};

mod domain;
mod host;
mod physical;
mod pm;

pub struct XenMetricsShared {
    pub physinfo: Option<xc_physinfo_t>,
    pub dominfos: Vec<xc_dominfo_t>,
    pub cpuinfos: Vec<xc_cpuinfo_t>,
    pub cpufreqs: Vec<u32>,
    pub pstates: Vec<PxStat>,
    pub cstates: Vec<CxStat>,
    pub xc: Rc<XenControl>,

    cpuinfos_buffer: Vec<MaybeUninit<xc_cpuinfo_t>>,
}

impl XenMetricsShared {
    pub fn new(xc: Rc<XenControl>) -> Self {
        let mut values = Self {
            xc,
            cpuinfos: vec![],
            cpuinfos_buffer: vec![],
            dominfos: vec![],
            cpufreqs: vec![],
            cstates: vec![],
            pstates: vec![],
            physinfo: None,
        };

        values.update();

        values
    }

    pub fn update(&mut self) {
        self.dominfos = (0..)
            .map_while(|i| self.xc.domain_getinfo(i).ok().flatten())
            .collect();

        self.physinfo = self.xc.physinfo().ok();

        let nr_cpus = self
            .physinfo
            .map(|physinfo| physinfo.nr_cpus as usize)
            .unwrap_or(0);

        self.cpuinfos_buffer
            .resize_with(nr_cpus, MaybeUninit::zeroed);

        self.cpufreqs.resize(nr_cpus, 0);
        self.cpufreqs
            .iter_mut()
            .enumerate()
            .for_each(|(cpuid, val)| {
                *val = self.xc.get_cpufreq_avg(cpuid as _).unwrap_or(0);
            });

        self.pstates.resize(nr_cpus, Default::default());
        self.pstates.iter_mut().enumerate().for_each(|(cpuid, px)| {
            if let Err(e) = self.xc.get_pxstat(cpuid as _, px) {
                println!("Unable to get PxStat info for CPU {cpuid} ({e})");
            }
        });

        self.cstates.resize(nr_cpus, Default::default());
        self.cstates.iter_mut().enumerate().for_each(|(cpuid, cx)| {
            if let Err(e) = self.xc.get_cxstat(cpuid as _, cx) {
                println!("Unable to get CxStat info for CPU {cpuid} ({e})");
            }
        });

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

    // pcpu avg
    metrics.push(Box::<PCpuAvg>::default());

    // CPU frequency
    metrics.push(Box::<CpuAvgFrequency>::default());

    // C-State info
    metrics.push(Box::<CpuCState>::default());

    // P-State info
    metrics.push(Box::<CpuPState>::default());

    // Memory infos
    metrics.push(Box::<MemoryTotal>::default());
    metrics.push(Box::<MemoryFree>::default());

    metrics.push(Box::<DomainMemory>::default());

    metrics.into_boxed_slice()
}
