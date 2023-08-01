mod metrics;

use metrics::{discover_xen_metrics, XenMetric, XenMetricsShared};
use std::{iter, time::Duration};
use tokio::time;

use xcp_metrics_common::rrdd::{
    protocol_common::{DataSourceMetadata, DataSourceValue},
    protocol_v2::{indexmap::IndexMap, RrddMetadata},
};
use xcp_metrics_plugin_common::protocol_v2::RrddPlugin;
use xenctrl::XenControl;

pub struct XenMetricsStatus {
    pub dom_count: u32,
}

/// Recreate a new metadata with new XenMetric objects.
fn regenerate_data_sources(
    shared: &XenMetricsShared,
) -> (Box<[Box<dyn XenMetric>]>, RrddMetadata, XenMetricsStatus) {
    let (metrics, status) = discover_xen_metrics(shared);

    let datasources: IndexMap<Box<str>, DataSourceMetadata> = metrics
        .iter()
        .map(|metric| {
            (
                metric.get_name().into(),
                metric.generate_metadata().unwrap_or_default(),
            )
        })
        .collect();

    (metrics, RrddMetadata { datasources }, status)
}

/// Create a array of values from XenMetric objects.
fn generate_values(sources: &mut [Box<dyn XenMetric>]) -> Box<[DataSourceValue]> {
    sources.iter().map(|src| src.get_value()).collect()
}

/// Replace the values with new ones from XenMetric objects.
fn generate_values_inplace(sources: &mut [Box<dyn XenMetric>], values: &mut [DataSourceValue]) {
    iter::zip(sources.iter(), values.iter_mut()).for_each(|(src, val)| *val = src.get_value());
}

/// Check for new domains.
fn check_new_metrics(xc: &XenControl, status: &XenMetricsStatus) -> bool {
    // Check next domain
    matches!(xc.domain_getinfo(status.dom_count + 1), Ok(Some(_)))
}

#[tokio::main]
async fn main() {
    let xc = xenctrl::XenControl::default().unwrap();
    let mut shared = XenMetricsShared::new(&xc);

    let (mut sources, mut metadata, mut status) = regenerate_data_sources(&shared);

    sources.iter_mut().for_each(|src| {
        src.update(&shared, &xc); // assume success
    });

    // NOTE: some could be undefined values
    let mut values = generate_values(&mut sources);

    let mut plugin = RrddPlugin::new("xcp-metrics-plugin-xen", metadata, Some(&values), None)
        .await
        .unwrap();

    // Expose protocol v2
    loop {
        // Update sources
        println!("Update sources");
        shared.update(&xc);

        if check_new_metrics(&xc, &status)
            || sources
                .iter_mut()
                .map(|src| src.update(&shared, &xc))
                .any(|b| !b)
        {
            println!("Rediscovering metrics...");

            // New metrics found or an update has failed, rediscover and regenerate metadata.
            (sources, metadata, status) = regenerate_data_sources(&shared);

            sources.iter_mut().for_each(|src| {
                src.update(&shared, &xc); // assume success
            });

            let values = generate_values(&mut sources);
            println!("Values vector: {values:?}");

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
