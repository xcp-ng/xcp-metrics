use bitflags::bitflags;
use uuid::Uuid;

use crate::{Align64, DomId};

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
#[derive(Clone, Copy, Debug, Default)]
pub struct XenDomctlGetVCpuInfo {
    // IN
    pub vcpu: u32,
    /// OUT: currently online (not hotplugged)?
    pub online: u8,
    /// OUT: blocked waiting for an event?
    pub blocked: u8,
    /// OUT: currently scheduled on its CPU?
    pub running: u8,
    /// OUT: total cpu time consumed (ns)
    pub cpu_time: Align64<u64>,
    /// OUT: current mapping
    pub cpu: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union XenDomctlParam {
    pub getdomaininfo: XenDomctlGetDomainInfo,
    pub getvcpuinfo: XenDomctlGetVCpuInfo,
    pub pad: [u8; 128],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct XenDomctl {
    pub cmd: u32,
    pub interface_version: u32,
    pub domain: DomId,
    pub pad: [u16; 3],
    pub param: XenDomctlParam,
}