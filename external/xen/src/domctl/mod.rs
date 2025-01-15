mod ffi;
pub use ffi::*;

use crate::{
    abi::get_xen_abi,
    hypercall::{XenHypercall, XenMutBuffer},
    DomId,
};

fn domctl_interface_version() -> u32 {
    match get_xen_abi() {
        crate::abi::XenAbi::Xen417 => 0x15,
        crate::abi::XenAbi::Xen419 => 0x17,
    }
}

const HYPERVISOR_DOMCTL: usize = 36;

const XEN_DOMCTL_GETDOMAININFO: u32 = 5;
const XEN_DOMCTL_GETVCPUINFO: u32 = 14;

pub trait DomctlGetDomainInfo
where
    Self: XenHypercall,
{
    fn get_domain_info(&self, domain: DomId) -> anyhow::Result<XenDomctlGetDomainInfo> {
        let mut domctl: XenDomctl = XenDomctl {
            cmd: XEN_DOMCTL_GETDOMAININFO,
            interface_version: domctl_interface_version(),
            domain,
            pad: [0; 3],
            param: XenDomctlParam {
                getdomaininfo: XenDomctlGetDomainInfo::default(),
            },
        };

        unsafe {
            let mut domctl_buffer = self.make_mut_buffer(&mut domctl)?;
            let res = self.hypercall1(HYPERVISOR_DOMCTL, domctl_buffer.as_hypercall_ptr() as usize);
            if res != 0 {
                anyhow::bail!("domctl_getdomaininfo failed {}", res as isize)
            }

            domctl_buffer.update();
        };

        Ok(unsafe { domctl.param.getdomaininfo })
    }
}

impl<T: XenHypercall> DomctlGetDomainInfo for T {}

pub trait DomctlGetVCpuInfo
where
    Self: XenHypercall,
{
    fn get_vcpu_info(&self, domain: DomId, vcpu: u32) -> anyhow::Result<XenDomctlGetVCpuInfo> {
        let mut domctl: XenDomctl = XenDomctl {
            cmd: XEN_DOMCTL_GETVCPUINFO,
            interface_version: domctl_interface_version(),
            domain,
            pad: [0; 3],
            param: XenDomctlParam {
                getvcpuinfo: XenDomctlGetVCpuInfo::default(),
            },
        };

        domctl.param.getvcpuinfo.vcpu = vcpu;

        unsafe {
            let mut domctl_buffer = self.make_mut_buffer(&mut domctl)?;
            let res = self.hypercall1(HYPERVISOR_DOMCTL, domctl_buffer.as_hypercall_ptr() as usize);
            if res != 0 {
                anyhow::bail!("domctl_getvcpuinfo failed {}", res as isize)
            }

            domctl_buffer.update();
        };

        Ok(unsafe { domctl.param.getvcpuinfo })
    }
}

impl<T: XenHypercall> DomctlGetVCpuInfo for T {}