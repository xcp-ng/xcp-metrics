use std::os::unix::net::UnixStream;

use xen::{domctl::DomctlGetDomainInfo, hypercall::XenHypercall, DomId};

pub fn run_plugin(mut stream: UnixStream, hyp: &impl XenHypercall) -> anyhow::Result<()> {
    let domains = hyp.get_domain_info(DomId(0))?;
    println!("{domains:?}");

    Ok(())
}
