mod ffi;
pub use ffi::*;

use crate::{
    abi::get_xen_abi,
    domctl::XenDomctlGetDomainInfo,
    hypercall::{XenHypercall, XenMutBuffer},
    Align64, DomId,
};

fn sysctl_interface_version() -> u32 {
    match get_xen_abi() {
        crate::abi::XenAbi::Xen417 => 0x15,
        crate::abi::XenAbi::Xen419 => 0x15,
    }
}

pub trait SysctlGetDomainInfoList
where
    Self: XenHypercall,
{
    fn get_domain_info_list(
        &self,
        first_domain: DomId,
    ) -> anyhow::Result<Vec<XenDomctlGetDomainInfo>> {
        let mut sysctl = XenSysctl {
            cmd: XEN_SYSCTL_GETDOMAININFOLIST,
            interface_version: sysctl_interface_version(),
            param: XenSysctlParam {
                getdomaininfolist: XenSysctlGetDomainInfoList::default(),
            },
        };

        unsafe {
            let mut domains = vec![XenDomctlGetDomainInfo::default(); 32];
            let domains_len = domains.len() as u32;

            let mut domain_infos = self.make_mut_slice(&mut domains)?;
            sysctl.param.getdomaininfolist = XenSysctlGetDomainInfoList {
                first_domain,
                max_domains: domains_len,
                buffer: Align64(domain_infos.as_hypercall_ptr()),
                num_domains: 0,
            };

            let mut sysctl_buffer = self.make_mut_buffer(&mut sysctl)?;
            let res = self.hypercall1(HYPERVISOR_SYSCTL, sysctl_buffer.as_hypercall_ptr() as usize);
            if res != 0 {
                anyhow::bail!("sysctl_getdomaininfolist failed {}", res as isize)
            }

            domain_infos.update();
            sysctl_buffer.update();
            drop(sysctl_buffer);
            drop(domain_infos);

            domains.truncate(sysctl.param.getdomaininfolist.num_domains as usize);
            Ok(domains)
        }
    }

    fn iter_domains(&self) -> DomainIterator<Self> {
        DomainIterator {
            hypercall: self,
            current_domid: DomId(0),
            domain_list: None,
        }
    }
}

impl<T: XenHypercall> SysctlGetDomainInfoList for T {}

pub struct DomainIterator<'a, H: XenHypercall> {
    hypercall: &'a H,
    current_domid: DomId,
    domain_list: Option<Vec<XenDomctlGetDomainInfo>>,
}

impl<H: XenHypercall> Iterator for DomainIterator<'_, H> {
    type Item = XenDomctlGetDomainInfo;

    fn next(&mut self) -> Option<Self::Item> {
        let domains = if let Some(domains) = self.domain_list.as_mut() {
            domains
        } else {
            self.domain_list = Some(
                self.hypercall
                    .get_domain_info_list(self.current_domid)
                    .inspect_err(|e| eprintln!("get_domain_info_list failure: {e}"))
                    .ok()?,
            );
            let domains = self.domain_list.as_mut().unwrap();

            // Set current_domid to maximum domid + 1
            let max_domid = domains
                .iter()
                .map(|dominf| dominf.domain)
                .max_by_key(|domid| domid.0)?;
            self.current_domid = DomId(max_domid.0 + 1);

            domains
        };

        let info = domains.pop();

        if domains.is_empty() {
            self.domain_list = None;
        }

        info
    }
}

pub trait SysctlPhysInfo
where
    Self: XenHypercall,
{
    fn physinfo(&self) -> anyhow::Result<XenSysctlPhysInfo> {
        let mut sysctl = XenSysctl {
            cmd: XEN_SYSCTL_PHYSINFO,
            interface_version: sysctl_interface_version(),
            param: XenSysctlParam {
                physinfo: XenSysctlPhysInfo::default(),
            },
        };

        let mut sysctl_buffer = self.make_mut_buffer(&mut sysctl)?;

        unsafe {
            let res = self.hypercall1(HYPERVISOR_SYSCTL, sysctl_buffer.as_hypercall_ptr() as usize);

            if res != 0 {
                anyhow::bail!("sysctl_getdomaininfolist failed {}", res as isize)
            }

            sysctl_buffer.update();
            drop(sysctl_buffer);

            Ok(sysctl.param.physinfo)
        }
    }
}

impl<H: XenHypercall> SysctlPhysInfo for H {}

pub trait SysctlGetCpuInfo
where
    Self: XenHypercall,
{
    fn get_cpu_info(&self, buffer: &mut [XenSysctlCpuinfo]) -> anyhow::Result<usize> {
        let max_cpus = buffer.len() as _;
        let mut cpu_infos_buffer = self.make_mut_slice(buffer)?;

        let mut sysctl = XenSysctl {
            cmd: XEN_SYSCTL_GETCPUINFO,
            interface_version: sysctl_interface_version(),
            param: XenSysctlParam {
                getcpuinfo: XenSysctlGetCpuInfo {
                    max_cpus,
                    info: Align64(cpu_infos_buffer.as_hypercall_ptr()),
                    nr_cpus: 0,
                },
            },
        };

        let mut sysctl_buffer = self.make_mut_buffer(&mut sysctl)?;

        unsafe {
            let res = self.hypercall1(HYPERVISOR_SYSCTL, sysctl_buffer.as_hypercall_ptr() as usize);

            if res != 0 {
                anyhow::bail!("sysctl_getdomaininfolist failed {}", res as isize)
            }

            sysctl_buffer.update();
            cpu_infos_buffer.update();
            drop(sysctl_buffer);
            drop(cpu_infos_buffer);

            Ok(sysctl.param.getcpuinfo.nr_cpus as _)
        }
    }
}

impl<H: XenHypercall> SysctlGetCpuInfo for H {}

pub trait SysctlGetPmOp
where
    Self: XenHypercall,
{
    fn get_cpufreq_avgfreq(&self, cpuid: u32) -> anyhow::Result<u64> {
        let mut sysctl = XenSysctl {
            cmd: XEN_SYSCTL_PM_OP,
            interface_version: sysctl_interface_version(),
            param: XenSysctlParam {
                pm_op: XenSysctlPmOp {
                    cmd: XEN_SYSCTL_PM_OP_CPUFREQ_AVG,
                    cpuid,
                    param: XenSysctlPmOpParam {
                        get_avgfreq: Align64(0),
                    },
                },
            },
        };

        unsafe {
            let mut sysctl_buffer = self.make_mut_buffer(&mut sysctl)?;
            let res = self.hypercall1(HYPERVISOR_SYSCTL, sysctl_buffer.as_hypercall_ptr() as _);

            if res != 0 {
                anyhow::bail!("sysctl_pm_op:cpufreq_avgfreq failed {}", res as isize)
            }

            sysctl_buffer.update();
            drop(sysctl_buffer);
            Ok(sysctl.param.pm_op.param.get_avgfreq.0)
        }
    }
}

impl<H: XenHypercall> SysctlGetPmOp for H {}
