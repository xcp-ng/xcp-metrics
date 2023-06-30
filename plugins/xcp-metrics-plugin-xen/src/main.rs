mod domain;
mod host;

use domain::{DomainMemory, VCpuTime};
use host::LoadAvg;
use std::{borrow::Cow, iter, rc::Rc, time::Duration};
use tokio::time;

use xcp_metrics_common::rrdd::{
    protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
    protocol_v2::{
        indexmap::{indexmap, IndexMap},
        RrddMetadata,
    },
};
use xcp_metrics_plugin_common::RrddPlugin;
use xenctrl::XenControl;
use xenctrl_sys::xc_dominfo_t;

pub trait XenMetric {
    /// Generate metadata for this metric.
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata>;

    /// Check if this metric still exists.
    fn update(&mut self) -> bool;

    /// Get the value of this metric.
    fn get_value(&self) -> DataSourceValue;

    /// Get the name of the metric.
    fn get_name(&self) -> Cow<str>;
}

const XEN_PAGE_SIZE: usize = 4096; // 4 KiB

fn discover_xen_metrics(xc: Rc<XenControl>) -> Box<[Box<dyn XenMetric>]> {
    let mut metrics: Vec<Box<dyn XenMetric>> = vec![];

    // Add loadavg metric.
    metrics.push(Box::new(LoadAvg::default()));

    for domid in 0.. {
        match xc.domain_getinfo(domid) {
            Ok(Some(xc_dominfo_t {
                handle,
                max_vcpu_id,
                ..
            })) => {
                // Domain exists

                // Domain memory
                let dom_uuid = uuid::Uuid::from_bytes(handle);

                metrics.push(Box::new(DomainMemory::new(xc.clone(), domid, dom_uuid)));

                // vCPUs
                for vcpuid in 0..=max_vcpu_id {
                    metrics.push(Box::new(VCpuTime::new(xc.clone(), vcpuid, domid, dom_uuid)));
                }
            }
            _ => break,
        }
    }

    metrics.into_boxed_slice()
}

fn regenerate_data_sources(xc: Rc<XenControl>) -> (Box<[Box<dyn XenMetric>]>, RrddMetadata) {
    let metrics = discover_xen_metrics(xc);

    let datasources: IndexMap<Box<str>, DataSourceMetadata> = metrics
        .iter()
        .map(|metric| {
            (
                metric.get_name().into(),
                metric.generate_metadata().unwrap_or_default(),
            )
        })
        .collect();

    (metrics, RrddMetadata { datasources })
}

fn generate_values(sources: &mut [Box<dyn XenMetric>]) -> Box<[DataSourceValue]> {
    sources.iter_mut().map(|src| src.get_value()).collect()
}

fn generate_values_inplace(sources: &mut [Box<dyn XenMetric>], values: &mut [DataSourceValue]) {
    iter::zip(sources.iter_mut(), values.iter_mut()).for_each(|(src, val)| *val = src.get_value());
}

#[tokio::main]
async fn main() {
    let xc = Rc::new(xenctrl::XenControl::default().unwrap());
    let (mut sources, mut metadata) = regenerate_data_sources(xc.clone());

    sources.iter_mut().for_each(|src| {
        src.update(); // assume success
    });

    // NOTE: some could be undefined values
    let mut values = generate_values(&mut sources);

    let mut plugin = RrddPlugin::new("xcp-metrics-plugin-xen", metadata, Some(&values))
        .await
        .unwrap();

    // Expose protocol v2
    loop {
        // Update sources

        //TODO: Detect new metrics.
        if sources.iter_mut().map(|src| src.update()).any(|b| !b) {
            // A update has failed, rediscover and regenerate metadata.
            (sources, metadata) = regenerate_data_sources(xc.clone());

            sources.iter_mut().for_each(|src| {
                src.update(); // assume success
            });

            let values = generate_values(&mut sources);

            plugin
                .reset_metadata(metadata, Some(&values))
                .await
                .unwrap();
        } else {
            // Update
            generate_values_inplace(&mut sources, &mut values);
        }

        plugin.update_values(&values).await.unwrap();
        time::sleep(Duration::from_secs(1)).await;
    }
}
