//! Utilities that manages communication with daemon using protocol v3
//! under the hood and converting to protocol v2 if needed.
use std::{collections::HashMap, time::Duration};

use tokio::time;

use crate::{
    bridge::v3_to_v2::BridgeToV2,
    protocol_v2::RrddPlugin,
    protocol_v3::{utils::SimpleMetricSet, MetricsPlugin},
};
use xcp_metrics_common::utils::mapping::CustomMapping;

/// Abstraction of a protocol v3 plugin.
pub trait XcpPlugin {
    /// Update the state of the plugin.
    fn update(&mut self);

    // Generate a new metric set representing the current state of data.
    fn generate_metrics(&mut self) -> SimpleMetricSet;
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
    mappings: HashMap<Box<str>, CustomMapping>,
    plugin_name: &str,
    target_daemon: Option<&str>,
    version: u32,
) {
    match version {
        2 => run_plugin_v2(shared, mappings, plugin_name, target_daemon).await,
        3 => run_plugin_v3(shared, plugin_name, target_daemon).await,
        p => tracing::error!("Unknown protocol {p}"),
    }
}

pub async fn run_plugin_v2(
    mut shared: impl XcpPlugin,
    mappings: HashMap<Box<str>, CustomMapping>,
    plugin_name: &str,
    target_daemon: Option<&str>,
) {
    tracing::info!("Running protocol v2 plugin: {plugin_name}");
    let mut metrics = shared.generate_metrics();

    let mut bridge = BridgeToV2::with_mappings(mappings);
    bridge.update(metrics.into());

    let mut plugin = match RrddPlugin::new(
        plugin_name,
        bridge.get_metadata().clone(),
        Some(&bridge.get_data()),
        target_daemon,
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

pub async fn run_plugin_v3(
    mut shared: impl XcpPlugin,
    plugin_name: &str,
    target_daemon: Option<&str>,
) {
    tracing::info!("Running protocol v3 plugin: {plugin_name}");
    // Expose protocol v3
    // NOTE: some could be undefined values
    let plugin = MetricsPlugin::new(plugin_name, shared.generate_metrics().into(), target_daemon)
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
