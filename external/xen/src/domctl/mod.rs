use bitflags::bitflags;
use uuid::Uuid;

use crate::{
    abi::get_xen_abi,
    hypercall::{XenHypercall, XenMutBuffer},
    Align64, DomId,
};

bitflags! {
  #[derive(Clone, Copy, Debug, Default)]
  pub struct XenDomctlDominf: u32 {
    /// Domain is scheduled to die.
    const DYING = 1;
    /// Domain is an HVM guest (as opposed to a PV guest).
    const HVM_GUEST = 1 << 1;
    /// The guest OS has shut down.
    const SHUTDOWN = 1 << 2;
    /// Currently paused by control software.
    const PAUSED = 1 << 3;
    /// Currently blocked pending an event.
    const BLOCKED = 1 << 4;
    /// Domain is currently running
    const RUNNING = 1 << 5;
    /// Being debugged.
    const DEBUGGED = 1 << 6;
    /// domain is a xenstore domain
    const XS_DOMAIN = 1 << 7;
    /// domain has hardware assisted paging
    const HAP = 1 << 8;
  }
}

bitflags! {
  /// Content of the `emulation_flags` field of the domain creation hypercall.
  #[repr(C)]
  #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
  pub struct XenX86Emu: u32 {
      /// Emulate Local APICs.
      const Lapic = 1 << 0;
      /// Emulate a HPET timer.
      const Hpet = 1 << 1;
      /// Emulate the ACPI PM timer.
      const Pm = 1 << 2;
      /// Emulate the RTC clock.
      const Rtc = 1 << 3;
      /// Emulate an IOAPIC device.
      const Ioapic = 1 << 4;
      /// Emulate PIC devices.
      const Pic = 1 << 5;
      /// Emulate standard VGA.
      const Vga = 1 << 6;
      /// Emulate an IOMMU.
      const Iommu = 1 << 7;
      /// Emulate a PIT timer.
      const Pit = 1 << 8;
      /// Route physical IRQs over event channels.
      const UsePirq = 1 << 9;
      /// Handle PCI configuration space traps from within Xen.
      const Vpci = 1 << 10;
  }
}

bitflags! {
  /// Contents of the `misc_flags` field of the domain creation hypercall
  #[repr(C)]
  #[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
  pub struct XenX86Misc: u32 {
      /// Grants access to the real physical MSR registers of the host.
      const MsrRelaxed = 1 << 0;
  }
}

/// x86-specific domain settings.
#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct XenArchDomainconfig {
    /// IN: Bitmap of devices to emulate.
    pub emulation_flags: XenX86Emu,
    /// IN: Miscellaneous x86-specific toggles.
    pub misc_flags: XenX86Misc,
}

const XEN_DOMCTL_GETDOMAININFO: u32 = 5;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct XenDomctlGetDomainInfo {
    pub domain: DomId,
    pub pad: u16,
    pub flags: XenDomctlDominf,
    pub tot_pages: Align64<u64>,
    pub max_pages: Align64<u64>,
    pub outstanding_pages: Align64<u64>,
    pub shr_pages: Align64<u64>,
    pub paged_pages: Align64<u64>,
    /// GMFN of shared_info struct
    pub shared_info_frame: Align64<u64>,
    pub cpu_time: Align64<u64>,
    /// Number of VCPUs currently online
    pub nr_online_vcpus: u32,
    /// Maximum VCPUID in use by this domain.
    pub max_vcpu_id: u32,
    pub ssidref: u32,
    pub handle: Uuid,
    pub cpupool: u32,
    pub gpaddr_bits: u8,
    pub pad2: [u8; 7],
    pub arch_config: XenArchDomainconfig,
}

#[repr(C)]
#[derive(Clone, Copy)]
union XenDomctlParam {
    pub getdomaininfo: XenDomctlGetDomainInfo,
    pub pad: [u8; 128],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct XenDomctl {
    pub cmd: u32,
    pub interface_version: u32,
    pub domain: DomId,
    pub pad: [u16; 3],
    pub param: XenDomctlParam,
}

fn domctl_interface_version() -> u32 {
    match get_xen_abi() {
        crate::abi::XenAbi::Xen417 => 0x15,
        crate::abi::XenAbi::Xen419 => 0x17,
    }
}

const HYPERVISOR_DOMCTL: usize = 36;

pub trait DomctlGetDomainInfo {
    fn get_domain_info(&self, domid: DomId) -> anyhow::Result<XenDomctlGetDomainInfo>;
}

impl<T: XenHypercall> DomctlGetDomainInfo for T {
    fn get_domain_info(&self, domain: DomId) -> anyhow::Result<XenDomctlGetDomainInfo> {
        let mut domctl = XenDomctl {
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
