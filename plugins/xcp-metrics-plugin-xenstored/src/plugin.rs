use std::collections::HashSet;

use xcp_metrics_plugin_common::xenstore::{
    watch_cache::WatchCache,
    xs::{XBTransaction, XsTrait},
};

pub struct PluginState {
    watch_cache: WatchCache,

    /// Domain ID -> Paths
    pub domains: HashSet<String>,

    /// VM ID -> Paths
    pub vms: HashSet<String>,
}

impl PluginState {
    pub fn new<XS: XsTrait + 'static>() -> Self {
        Self {
            watch_cache: WatchCache::new::<XS>(),
            domains: HashSet::default(),
            vms: HashSet::default(),
        }
    }
}

static TRACKED_DOMAIN_ATTRIBUTES: &[&str] = &["memory/target", "vm"];
static TRACKED_VM_ATTRIBUTES: &[&str] = &["name", "uuid"];

impl PluginState {
    fn track_domain(&mut self, domain: &str) {
        TRACKED_DOMAIN_ATTRIBUTES.iter().for_each(|attribute| {
            if let Err(e) = self
                .watch_cache
                .watch(format!("/local/domain/{domain}/{attribute}").as_str())
            {
                println!("{e}");
            }
        });

        self.domains.insert(domain.to_string());
    }

    fn untrack_domain(&mut self, domain: &str) {
        TRACKED_DOMAIN_ATTRIBUTES.iter().for_each(|attribute| {
            if let Err(e) = self
                .watch_cache
                .unwatch(format!("/local/domain/{domain}/{attribute}").as_str())
            {
                println!("{e}");
            }
        });

        self.domains.remove(domain);
    }

    fn track_vm(&mut self, vm: &str) {
        TRACKED_VM_ATTRIBUTES.iter().for_each(|attribute| {
            if let Err(e) = self
                .watch_cache
                .watch(format!("/vm/{vm}/{attribute}").as_str())
            {
                println!("{e}");
            }
        });

        self.vms.insert(vm.to_string());
    }

    fn untrack_vm(&mut self, vm: &str) {
        TRACKED_VM_ATTRIBUTES.iter().for_each(|attribute| {
            if let Err(e) = self
                .watch_cache
                .unwatch(format!("/vm/{vm}/{attribute}").as_str())
            {
                println!("{e}");
            }
        });

        self.vms.remove(vm);
    }

    /// Check for removed and new domains, and update watcher.
    pub fn update_domains<XS: XsTrait>(&mut self, xs: &XS) -> anyhow::Result<()> {
        let real_domains: HashSet<String> = xs
            .directory(XBTransaction::Null, "/local/domain")?
            .into_iter()
            .collect();

        real_domains.iter().for_each(|domain| {
            if !self.domains.contains(domain) {
                println!("Now tracking domain {domain}");
                self.track_domain(domain);
            }
        });

        // Check for removed domains.
        self.domains
            .difference(&real_domains)
            .cloned()
            .collect::<Vec<String>>()
            .into_iter()
            .for_each(|domain| {
                println!("Untracking domain {domain}");
                self.untrack_domain(&domain);
            });

        Ok(())
    }

    /// Check for removed and new vms, and update watcher.
    pub fn update_vms<XS: XsTrait>(&mut self, xs: &XS) -> anyhow::Result<()> {
        let real_vms: HashSet<String> = xs
            .directory(XBTransaction::Null, "/vm")?
            .into_iter()
            .collect();

        real_vms.iter().for_each(|vm| {
            if !self.vms.contains(vm) {
                println!("Now tracking vm {vm}");
                self.track_vm(vm);
            }
        });

        // Check removed domains.
        self.vms
            .difference(&real_vms)
            .cloned()
            .collect::<Vec<String>>()
            .into_iter()
            .for_each(|vm| {
                println!("Untracking vm {vm}");
                self.untrack_vm(&vm);
            });

        Ok(())
    }

    pub fn read(&self, path: &str) -> Option<String> {
        self.watch_cache.read(path)
    }
}
