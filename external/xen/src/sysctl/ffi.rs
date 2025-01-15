use crate::{domctl::XenDomctlGetDomainInfo, Align64, DomId};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct XenSysctlGetDomainInfoList {
    /// IN
    pub first_domain: DomId,
    /// IN
    pub max_domains: u32,
    /// IN
    pub buffer: Align64<*mut XenDomctlGetDomainInfo>,
    /// OUT variables.
    pub num_domains: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct XenSysctlPhysInfo {
    pub threads_per_core: u32,
    pub cores_per_socket: u32,
    pub nr_cpus: u32,
    pub max_cpu_id: u32,
    pub nr_nodes: u32,
    pub max_node_id: u32,
    pub cpu_khz: u32,
    pub capabilities: u32,
    pub arch_capabilities: u32,
    pub pad: u32,
    pub total_pages: Align64<u64>,
    pub free_pages: Align64<u64>,
    pub scrub_pages: Align64<u64>,
    pub outstanding_pages: Align64<u64>,
    pub max_mfn: Align64<u64>,
    pub hw_cap: [u32; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct XenSysctlCpuinfo {
    pub idletime: Align64<u64>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct XenSysctlGetCpuInfo {
    /// IN
    pub max_cpus: u32,
    /// IN
    pub info: Align64<*mut XenSysctlCpuinfo>,
    /// OUT
    pub nr_cpus: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union XenSysctlPmOpParam {
    pub get_avgfreq: Align64<u64>,
    _pad: [u8; 128], // Just to make sure we are large enough
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XenSysctlPmOp {
    pub cmd: u32,
    pub cpuid: u32,
    pub param: XenSysctlPmOpParam,
}

// GET_CPUFREQ_AVGFREQ = CPUFREQ_PARA | 0x04
pub const XEN_SYSCTL_PM_OP_CPUFREQ_AVG: u32 = 0x10 | 0x04;

#[repr(C)]
#[derive(Clone, Copy)]
pub union XenSysctlParam {
    pub getdomaininfolist: XenSysctlGetDomainInfoList,
    pub physinfo: XenSysctlPhysInfo,
    pub getcpuinfo: XenSysctlGetCpuInfo,
    pub pm_op: XenSysctlPmOp,
    _pad: [u8; 128],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XenSysctl {
    pub cmd: u32,
    pub interface_version: u32,
    pub param: XenSysctlParam,
}

pub const HYPERVISOR_SYSCTL: usize = 35;

pub const XEN_SYSCTL_PHYSINFO: u32 = 3;
pub const XEN_SYSCTL_GETDOMAININFOLIST: u32 = 6;
pub const XEN_SYSCTL_GETCPUINFO: u32 = 8;
pub const XEN_SYSCTL_PM_OP: u32 = 12;
