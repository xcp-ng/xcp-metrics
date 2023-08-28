use std::{collections::HashMap, path::Path, time::Duration};

use clap::{command, Parser};
use tokio::time;
use xapi::METRICS_SHM_PATH;
use xcp_metrics_common::{
    metrics::MetricSet,
    protocol_v3::{self, ProtocolV3Header},
    utils::mapping::CustomMapping,
};

use xcp_metrics_plugin_common::{bridge::v3_to_v2::BridgeToV2, protocol_v2::RrddPlugin};

/// A bridge plugin (from v3 to v2) for protocol v3 plugins.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Name of the plugin to bridge.
    #[arg(short, long)]
    plugin_name: String,

    /// Logging level
    #[arg(short, long, default_value_t = tracing::Level::INFO)]
    log_level: tracing::Level,

    /// Target daemon path.
    #[arg(short, long, default_value_t = String::from("/var/lib/xcp/xcp-metrics"))]
    target: String,

    /// V3 to V2 mapping file (JSON format)
    #[arg(short, long)]
    mapping_path: Option<String>,
}

fn load_mapping(args: &Args) -> HashMap<Box<str>, CustomMapping> {
    if let Some(path) = &args.mapping_path {
        let content = std::fs::read_to_string(path).expect("Unable to read mapping file");
        serde_json::from_str(&content).expect("Invalid mapping file.")
    } else {
        Default::default()
    }
}

async fn read_protocol_v3(path: &str) -> anyhow::Result<(ProtocolV3Header, MetricSet)> {
    let mut reader = tokio::fs::File::open(path).await?;

    Ok(protocol_v3::parse_v3_async(&mut reader).await?)
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(args.log_level)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let bridged_plugin_name = format!("{}_bridged", args.plugin_name);

    let path = format!("{METRICS_SHM_PATH}{}", args.plugin_name);
    tracing::info!("Plugin to bridge path: {path}");

    let (header, metrics_set) = read_protocol_v3(&path).await.unwrap();

    tracing::debug!("Protocol v3 header: {header:?}");
    tracing::debug!("Initial MetricsSet: {metrics_set:?}");

    let mut bridge = BridgeToV2::with_mappings(load_mapping(&args));
    bridge.update(metrics_set);

    let mut plugin = RrddPlugin::new(
        &bridged_plugin_name,
        bridge.get_metadata().clone(),
        Some(&bridge.get_data()),
        Some(&Path::new(&args.target)),
    )
    .await
    .unwrap();

    // Expose protocol v2
    loop {
        let (header, metrics_set) = read_protocol_v3(&path).await.unwrap();
        tracing::debug!("Updated: {header:?}");
        tracing::debug!(" - {metrics_set:?}");

        // Update sources
        if bridge.update(metrics_set) {
            tracing::debug!("Updating metadata");
            let metadata = bridge.get_metadata().clone();
            tracing::debug!(" - {metadata:?}");

            plugin
                .reset_metadata(metadata, Some(&bridge.get_data()))
                .await
                .unwrap();
        }

        plugin.update_values(&bridge.get_data()).await.unwrap();
        time::sleep(Duration::from_secs(1)).await;
    }
}
