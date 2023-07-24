mod metrics;

use metrics::{discover_xen_metrics, XenMetric, XenMetricsShared};
use std::{rc::Rc, time::Duration};
use tokio::time;

use xcp_metrics_plugin_common::protocol_v3::{utils::SimpleMetricSet, MetricsPlugin};

pub fn generate_metrics(
    shared: &XenMetricsShared,
    sources: &mut Box<[Box<dyn XenMetric>]>,
) -> SimpleMetricSet {
    SimpleMetricSet {
        families: sources
            .iter_mut()
            .filter_map(|source| source.get_family(shared))
            .map(|(name, family)| (name.into_string(), family))
            .collect(),
    }
}

#[tokio::main]
async fn main() {
    let xc = Rc::new(xenctrl::XenControl::default().unwrap());
    let mut sources = discover_xen_metrics(xc.clone());
    let mut shared = XenMetricsShared::new(xc);

    // NOTE: some could be undefined values
    let metrics = generate_metrics(&shared, &mut sources);

    let plugin = MetricsPlugin::new("xcp-metrics-plugin-xen", metrics.clone().into())
        .await
        .unwrap();

    // Expose protocol v2
    loop {
        // Update sources
        shared.update();

        // Fetch and push new metrics.
        plugin
            .update(generate_metrics(&shared, &mut sources).into())
            .await
            .unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }
}
