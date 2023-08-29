//! Utilities that manages communication with daemon using protocol v3
//! under the hood and converting to protocol v2 if needed.
use std::{collections::HashMap, path::Path, time::Duration};

use tokio::time;

use crate::{
    bridge::v3_to_v2::BridgeToV2,
    protocol_v2::RrddPlugin,
    protocol_v3::{utils::SimpleMetricSet, MetricsPlugin},
};
use xcp_metrics_common::utils::mapping::CustomMapping;

pub const XCP_RRDD_PATH: &str = "/var/lib/xcp/xcp-rrdd";

/// Abstraction of a protocol v3 plugin.
pub trait XcpPlugin {
    /// Update the state of the plugin.
    fn update(&mut self);

    // Generate a new metric set representing the current state of data.
    fn generate_metrics(&mut self) -> SimpleMetricSet;

    /// Get the plugin name (uid).
    fn get_name(&self) -> &str;

    // Get plugin mappings
    fn get_mappings(&self) -> Option<HashMap<Box<str>, CustomMapping>>;
}

fn arg0_starts_with_rrdp() -> bool {
    let Some(arg0) = std::env::args().next() else {
        return false;
    };

    Path::new(&arg0)
        .file_name()
        .unwrap_or_default()
        .to_os_string()
        .to_string_lossy()
        .starts_with("rrdp-")
}

/// Run the provided for either protocol v2 (converting from v3) or protocol v3 depending on `version`.
///
/// Versions :
///   2: Use a internal v3 to v2 bridge with `mappings` to convert from protocol v3 to protocol v2.
///   3: Directly expose protocol v3 metrics to target daemon.
///
/// If no target_daemon is provided, use default one.
pub async fn run_hybrid(
    shared: impl XcpPlugin,
    mut target_daemon_path: Option<&Path>,
    mut version: Option<u32>,
) {
    if target_daemon_path.is_none() && version.is_none() && arg0_starts_with_rrdp() {
        tracing::info!("Program name starts with rrdp-*, use xcp-rrdd and protocol-v2 by default.");
        target_daemon_path = Some(Path::new(&XCP_RRDD_PATH));
        version = Some(2);
    }

    match version.unwrap_or(3) {
        2 => run_plugin_v2(shared, target_daemon_path).await,
        3 => run_plugin_v3(shared, target_daemon_path).await,
        p => tracing::error!("Unknown protocol {p}"),
    }
}

pub async fn run_plugin_v2(mut shared: impl XcpPlugin, target_daemon_path: Option<&Path>) {
    tracing::info!("Running protocol v2 plugin: {}", shared.get_name());
    let mut metrics = shared.generate_metrics();

    let mut bridge = BridgeToV2::with_mappings(shared.get_mappings().unwrap_or_default());
    bridge.update(metrics.into());

    let mut plugin = match RrddPlugin::new(
        shared.get_name(),
        bridge.get_metadata().clone(),
        Some(&bridge.get_data()),
        target_daemon_path,
    )
    .await
    {
        Ok(plugin) => plugin,
        Err(e) => {
            tracing::error!("Unable to initialize plugin ({e})");
            return;
        }
    };

    // Expose protocol v2
    loop {
        tracing::debug!("Updating plugin state");

        // Update sources
        shared.update();

        // Fetch and push new metrics.
        metrics = shared.generate_metrics();

        if bridge.update(metrics.into()) {
            if let Err(e) = plugin
                .reset_metadata(bridge.get_metadata().clone(), Some(&bridge.get_data()))
                .await
            {
                tracing::warn!("Unable to update metadata ({e}");
            }
        }

        if let Err(e) = plugin.update_values(&bridge.get_data()).await {
            tracing::warn!("Unable to update plugin values ({e})");
        }

        time::sleep(Duration::from_secs(1)).await;
    }
}

pub async fn run_plugin_v3(mut shared: impl XcpPlugin, target_daemon_path: Option<&Path>) {
    tracing::info!("Running protocol v3 plugin: {}", shared.get_name());
    // Expose protocol v3
    // NOTE: some could be undefined values
    let plugin = MetricsPlugin::new(
        &shared.get_name().to_string(),
        shared.generate_metrics().into(),
        target_daemon_path,
    )
    .await
    .unwrap();

    loop {
        tracing::debug!("Updating plugin state");
        // Update sources
        shared.update();

        // Fetch and push new metrics.
        plugin
            .update(shared.generate_metrics().into())
            .await
            .unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }
}
