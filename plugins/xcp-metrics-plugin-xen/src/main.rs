mod metrics;

use metrics::{discover_xen_metrics, XenMetric};
use std::{iter, rc::Rc, time::Duration};
use tokio::time;

use xcp_metrics_common::rrdd::{
    protocol_common::{DataSourceMetadata, DataSourceValue},
    protocol_v2::{indexmap::IndexMap, RrddMetadata},
};
use xcp_metrics_plugin_common::RrddPlugin;
use xenctrl::XenControl;

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
