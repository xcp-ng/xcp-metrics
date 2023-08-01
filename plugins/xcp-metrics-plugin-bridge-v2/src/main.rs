use clap::{command, Parser};
use std::time::Duration;
use tokio::time;
use xcp_metrics_common::{
    metrics::MetricSet,
    protocol_v3::{self, ProtocolV3Header},
    xapi::METRICS_SHM_PATH,
};

use xcp_metrics_plugin_common::{bridge::v3_to_v2::BridgeToV2, protocol_v2::RrddPlugin};

/// A bridge plugin (from v3 to v2) for protocol v3 plugins.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the plugin to bridge.
    #[arg(short, long)]
    plugin_name: String,

    /// Target daemon.
    #[arg(short, long, default_value_t = String::from("xcp-metrics"))]
    target: String,
}

async fn read_protocol_v3(path: &str) -> anyhow::Result<(ProtocolV3Header, MetricSet)> {
    let mut reader = tokio::fs::File::open(path).await?;

    Ok(protocol_v3::parse_v3_async(&mut reader).await?)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let bridged_plugin_name = format!("{}_bridged", args.plugin_name);

    let path = format!("{METRICS_SHM_PATH}{}", args.plugin_name);
    println!("{path}");

    let (header, metrics_set) = read_protocol_v3(&path).await.unwrap();

    println!("Protocol v3 header: {header:?}");
    println!("Initial MetricsSet: {metrics_set:?}");

    let mut bridge = BridgeToV2::default();
    bridge.update(metrics_set);

    let mut plugin = RrddPlugin::new(
        &bridged_plugin_name,
        bridge.get_metadata().clone(),
        Some(&bridge.get_data()),
        Some(&args.target),
    )
    .await
    .unwrap();

    // Expose protocol v2
    loop {
        let (header, metrics_set) = read_protocol_v3(&path).await.unwrap();
        println!("Updated: {header:?}");
        println!(" - {metrics_set:?}");

        // Update sources
        if bridge.update(metrics_set) {
            println!("Updating metadata");
            let metadata = bridge.get_metadata().clone();
            println!(" - {metadata:?}");

            plugin
                .reset_metadata(metadata, Some(&bridge.get_data()))
                .await
                .unwrap();
        }

        plugin.update_values(&bridge.get_data()).await.unwrap();
        time::sleep(Duration::from_secs(1)).await;
    }
}
