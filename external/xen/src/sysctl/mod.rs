use crate::{
    abi::get_xen_abi,
    domctl::XenDomctlGetDomainInfo,
    hypercall::{XenHypercall, XenMutBuffer},
    Align64, DomId,
};

#[repr(C)]
#[derive(Default, Clone, Copy)]
struct XenSysctlGetDomainInfoList {
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
#[derive(Clone, Copy)]
union XenSysctlParam {
    pub getdomaininfolist: XenSysctlGetDomainInfoList,
    pub pad: [u8; 128],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XenSysctl {
    pub cmd: u32,
    pub interface_version: u32,
    pub param: XenSysctlParam,
}

fn sysctl_interface_version() -> u32 {
    match get_xen_abi() {
        crate::abi::XenAbi::Xen417 => 0x15,
        crate::abi::XenAbi::Xen419 => 0x17,
    }
}

const HYPERVISOR_SYSCTL: usize = 35;

// TODO: Make it a iterator over spans of domaininfo.
//       See init-xenstore-domain.c:check_domain()
pub trait SysctlGetDomainInfoList {
    const CMD: u32 = 6;

    fn get_domain_info_list(&self) -> anyhow::Result<Vec<XenDomctlGetDomainInfo>>;
}

impl<T: XenHypercall> SysctlGetDomainInfoList for T {
    fn get_domain_info_list(&self) -> anyhow::Result<Vec<XenDomctlGetDomainInfo>> {
        let mut sysctl = XenSysctl {
            cmd: Self::CMD,
            interface_version: sysctl_interface_version(),
            param: XenSysctlParam {
                getdomaininfolist: XenSysctlGetDomainInfoList::default(),
            },
        };

        unsafe {
            let mut sysctl_buffer = self.make_mut_buffer(&mut sysctl)?;
            let res = self.hypercall1(HYPERVISOR_SYSCTL, sysctl_buffer.as_hypercall_ptr() as usize);
            if res != 0 {
                anyhow::bail!("sysctl_getdomaininfolist failed {}", res as isize)
            }

            sysctl_buffer.update();
            drop(sysctl_buffer);

            let mut domains: Vec<XenDomctlGetDomainInfo> = vec![
                XenDomctlGetDomainInfo::default();
                sysctl.param.getdomaininfolist.num_domains
                    as usize
            ];
            let domains_len = domains.len() as u32;

            let mut domain_infos = self.make_mut_slice(domains.as_mut_slice())?;
            sysctl.param.getdomaininfolist = XenSysctlGetDomainInfoList {
                first_domain: DomId(0),
                max_domains: domains_len,
                buffer: Align64(domain_infos.as_hypercall_ptr()),
                num_domains: domains_len,
            };

            let mut sysctl_buffer = self.make_mut_buffer(&mut sysctl)?;
            let res = self.hypercall1(HYPERVISOR_SYSCTL, sysctl_buffer.as_hypercall_ptr() as usize);
            if res != 0 {
                anyhow::bail!("sysctl_getdomaininfolist failed {}", res as isize)
            }
            domain_infos.update();
            drop(domain_infos);

            Ok(domains)
        }
    }
}
