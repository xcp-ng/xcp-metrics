/// That's a hack.
use std::{fs, sync::LazyLock};

#[derive(Clone, Copy)]
pub enum XenAbi {
    /// Xen 4.17 (XCP-ng 8.3)
    Xen417,
    /// Xen 4.19 (upstream Xen)
    Xen419,
}

static ONCE_ABI: LazyLock<XenAbi> = LazyLock::new(|| {
    let major = fs::read_to_string("/sys/hypervisor/version/major").expect("Not running under Xen");
    let minor = fs::read_to_string("/sys/hypervisor/version/minor")
        .expect("Unable to read minor Xen version.");

    let major = major.trim();
    let minor = minor.trim();

    match (major, minor) {
        ("4", "19") => XenAbi::Xen419,
        ("4", "17") => XenAbi::Xen417,
        _ => panic!("Unsupported Xen version {major}.{minor}"),
    }
});

pub fn get_xen_abi() -> XenAbi {
    *ONCE_ABI
}
