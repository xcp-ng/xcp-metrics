use std::collections::{HashMap, HashSet};

use xcp_metrics_common::metrics::{Label, MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::{
    protocol_v3::utils::{SimpleMetric, SimpleMetricFamily, SimpleMetricSet},
    plugin::XcpPlugin,
    xenstore::{
        watch_cache::WatchCache,
        xs::{XBTransaction, Xs, XsOpenFlags, XsTrait},
    },
};

pub struct XenStorePlugin<'a, XS: XsTrait> {
    watch_cache: WatchCache,
    xs: &'a XS,

    /// Domain ID -> Paths
    pub domains: HashSet<String>,

    /// VM ID -> Paths
    pub vms: HashSet<String>,
}

impl<'a, XS: XsTrait> XenStorePlugin<'a, XS> {
    pub fn new(xs: &'a XS) -> Self {
        Self {
            xs,
            watch_cache: WatchCache::new(
                Xs::new(XsOpenFlags::ReadOnly).expect("Unable to create second Xs"),
            ),
            domains: HashSet::default(),
            vms: HashSet::default(),
        }
    }
}

static TRACKED_DOMAIN_ATTRIBUTES: &[&str] = &["memory/target", "vm"];
static TRACKED_VM_ATTRIBUTES: &[&str] = &["name", "uuid"];

impl<XS: XsTrait> XenStorePlugin<'_, XS> {
    pub fn get_vm_infos(&self, vm_uuid: &str, attributes: &[&str]) -> MetricValue {
        MetricValue::Info(
            attributes
                .iter()
                .filter_map(|&attr| {
                    self.read(format!("/vm/{vm_uuid}/{attr}").as_str())
                        .map(|value| Label(attr.into(), value.into()))
                })
                .collect(),
        )
    }

    fn make_memory_target_metric(&self, domid: &str, memory_target: i64) -> SimpleMetric {
        let vm_uuid = self.get_domain_uuid(domid);

        let mut labels = vec![Label("domain".into(), domid.into())];

        if let Some(vm_uuid) = vm_uuid {
            labels.push(Label("owner".into(), format!("vm {vm_uuid}").into()));
        }

        SimpleMetric {
            labels,
            value: MetricValue::Gauge(NumberValue::Int64(memory_target)),
        }
    }

    fn get_domain_uuid(&self, domid: &str) -> Option<String> {
        self.read(format!("/local/domain/{domid}/vm").as_str())
            .and_then(|vm_path| self.read(format!("{vm_path}/uuid").as_str()))
    }

    fn get_memory_target_value(&self, domid: &str) -> Option<i64> {
        self.read(format!("/local/domain/{domid}/memory/target").as_str())
            .and_then(|value| {
                value
                    .parse()
                    .map_err(|err| {
                        tracing::error!("Memory target parse error {err:?}");
                        err
                    })
                    .ok()
            })
    }

    fn track_domain(&mut self, domain: &str) {
        TRACKED_DOMAIN_ATTRIBUTES.iter().for_each(|attribute| {
            if let Err(e) = self
                .watch_cache
                .watch(format!("/local/domain/{domain}/{attribute}").as_str())
            {
                tracing::warn!("Unable to watch domain attribute ({e})");
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
                tracing::warn!("Unable to unwatch domain attribute ({e})");
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
                tracing::warn!("Unable to watch vm attribute ({e})");
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
                tracing::warn!("Unable to unwatch vm attribute ({e})");
            }
        });

        self.vms.remove(vm);
    }

    /// Check for removed and new domains, and update watcher.
    pub fn update_domains(&mut self) -> anyhow::Result<()> {
        let real_domains: HashSet<String> = self
            .xs
            .directory(XBTransaction::Null, "/local/domain")?
            .into_iter()
            .collect();

        real_domains.iter().for_each(|domain| {
            if !self.domains.contains(domain) {
                tracing::debug!("Now tracking domain {domain}");
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
                tracing::debug!("Untracking domain {domain}");
                self.untrack_domain(&domain);
            });

        Ok(())
    }

    /// Check for removed and new vms, and update watcher.
    pub fn update_vms(&mut self) -> anyhow::Result<()> {
        let real_vms: HashSet<String> = self
            .xs
            .directory(XBTransaction::Null, "/vm")?
            .into_iter()
            .collect();

        real_vms.iter().for_each(|vm| {
            if !self.vms.contains(vm) {
                tracing::debug!("Now tracking vm {vm}");
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
                tracing::debug!("Untracking vm {vm}");
                self.untrack_vm(&vm);
            });

        Ok(())
    }

    pub fn read(&self, path: &str) -> Option<String> {
        self.watch_cache.read(path)
    }
}

impl<XS: XsTrait> XcpPlugin for XenStorePlugin<'_, XS> {
    fn update(&mut self) {
        if let Err(e) = self.update_domains() {
            tracing::warn!("Unable to get domains: {e}");
        }

        if let Err(e) = self.update_vms() {
            tracing::warn!("Unable to get vms: {e}");
        }
    }

    fn generate_metrics(&mut self) -> SimpleMetricSet {
        let mut families: HashMap<String, SimpleMetricFamily> = HashMap::new();

        families.insert(
            "vm_info".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Info,
                unit: "".into(),
                help: "Virtual machine informations".into(),
                metrics: self
                    .vms
                    .iter()
                    // Get vm metrics.
                    .map(|uuid| SimpleMetric {
                        labels: vec![Label("owner".into(), format!("vm {uuid}").into())],
                        value: self.get_vm_infos(uuid, &["name"]),
                    })
                    .collect(),
            },
        );

        families.insert(
            "memory_target".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "bytes".into(),
                help: "Target of VM balloon driver".into(),
                metrics: self
                    .domains
                    .iter()
                    // Get target memory metric (if exists).
                    .filter_map(|domid| self.get_memory_target_value(domid).map(|m| (domid, m)))
                    // Make it a metric.
                    .map(|(domid, memory_target)| {
                        self.make_memory_target_metric(domid, memory_target)
                    })
                    .collect(),
            },
        );

        SimpleMetricSet { families }
    }
}
