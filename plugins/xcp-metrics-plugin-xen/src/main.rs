mod metrics;

use clap::Parser;
use metrics::{discover_xen_metrics, XenMetric, XenMetricsShared};
use std::{rc::Rc, time::Duration};
use tokio::time;

use xcp_metrics_plugin_common::{
    bridge::v3_to_v2::BridgeToV2,
    protocol_v2::RrddPlugin,
    protocol_v3::{utils::SimpleMetricSet, MetricsPlugin},
};

/// OpenMetrics http proxy, used to provide metrics for collectors such as Prometheus.
#[derive(Clone, Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Target daemon.
    #[arg(short, long, default_value_t = String::from("xcp-metrics"))]
    target: String,

    /// Used protocol
    #[arg(short, long, default_value_t = 3)]
    protocol: u32,
}

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
    let args = Args::parse();

    let xc = Rc::new(xenctrl::XenControl::default().unwrap());
    let sources = discover_xen_metrics(xc.clone());
    let shared = XenMetricsShared::new(xc);

    match args.protocol {
        2 => plugin_v2(args, sources, shared).await,
        3 => plugin_v3(args, sources, shared).await,

        p => eprintln!("Unsupported protocol ({p})"),
    }
}

async fn plugin_v2(
    args: Args,
    mut sources: Box<[Box<dyn XenMetric>]>,
    mut shared: XenMetricsShared,
) {
    let mut metrics = generate_metrics(&shared, &mut sources);

    let mut bridge = BridgeToV2::default();
    bridge.update(metrics.into());

    let mut plugin = RrddPlugin::new(
        "xcp-metrics-plugin-xen",
        bridge.get_metadata().clone(),
        Some(&bridge.get_data()),
        Some(&args.target),
    )
    .await
    .unwrap();

    // Expose protocol v2
    loop {
        // Update sources
        shared.update();

        // Fetch and push new metrics.
        metrics = generate_metrics(&shared, &mut sources);

        if bridge.update(metrics.into()) {
            plugin
                .reset_metadata(bridge.get_metadata().clone(), Some(&bridge.get_data()))
                .await
                .unwrap();
        }

        plugin.update_values(&bridge.get_data()).await.unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }
}

async fn plugin_v3(
    args: Args,
    mut sources: Box<[Box<dyn XenMetric>]>,
    mut shared: XenMetricsShared,
) {
    // Expose protocol v3
    // NOTE: some could be undefined values
    let metrics = generate_metrics(&shared, &mut sources);

    let plugin = MetricsPlugin::new(
        "xcp-metrics-plugin-xen",
        metrics.clone().into(),
        Some(&args.target),
    )
    .await
    .unwrap();

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
