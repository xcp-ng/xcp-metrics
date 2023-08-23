//! xcp-metrics plugin protocol v3 framework.
pub mod utils;

use std::path::{Path, PathBuf};

use tokio::fs::{create_dir_all, OpenOptions};
use xcp_metrics_common::{
    metrics::MetricSet,
    protocol_v3,
    rpc::methods::{PluginLocalDeregister, PluginLocalRegister},
    xapi::{self, hyper::body::HttpBody, METRICS_SHM_PATH},
};

pub struct MetricsPlugin {
    uid: Box<str>,
    metrics_path: PathBuf,
    target_daemon: Box<str>,
}

const DEFAULT_DAEMON: &str = "xcp-metrics";

impl MetricsPlugin {
    /// Create and register a new plugin.
    pub async fn new(
        uid: &'_ str,
        metrics: MetricSet,
        target_daemon: Option<&str>,
    ) -> anyhow::Result<Self> {
        let plugin = Self {
            uid: uid.into(),
            metrics_path: Path::new(METRICS_SHM_PATH).join(uid),
            target_daemon: target_daemon.unwrap_or(DEFAULT_DAEMON).into(),
        };

        plugin.update(metrics).await?;
        plugin.advertise_plugin().await?;

        Ok(plugin)
    }

    pub async fn update(&self, metrics: MetricSet) -> anyhow::Result<()> {
        create_dir_all(METRICS_SHM_PATH).await?;

        let mut options = OpenOptions::new();
        options.create(true);
        options.truncate(false);
        options.write(true);

        let mut file = options.open(&self.metrics_path).await?;

        protocol_v3::generate_v3_async(&mut file, None, metrics).await?;

        Ok(())
    }

    /// Advertise the existence of the plugin to the main daemon.
    pub async fn advertise_plugin(&self) -> anyhow::Result<()> {
        let request = PluginLocalRegister {
            info: "Five_Seconds".into(),
            protocol: "V3".into(),
            uid: (*self.uid).into(),
        };

        let response = xapi::send_xmlrpc_at(
            &self.target_daemon,
            "POST",
            &request,
            &self.uid, /* use uid as user-agent */
        )
        .await?;

        tracing::debug!("RPC Response: {response:?}");
        if let Some(Ok(body)) = response.into_body().data().await {
            tracing::debug!("RPC Body:\n{:}", String::from_utf8_lossy(&body));
        }

        Ok(())
    }

    /// Deregister the plugin from the daemon.
    pub async fn deregister_plugin(self) {
        tracing::debug!("Deregistering {}...", &self.uid);

        // Unregister plugin
        let request = PluginLocalDeregister {
            uid: self.uid.to_string(),
        };

        let response = xapi::send_xmlrpc_at(
            &self.target_daemon,
            "POST",
            &request,
            &self.uid, /* use uid as user-agent */
        )
        .await
        .unwrap();

        tracing::debug!("RPC Response: {response:?}");
        if let Some(Ok(body)) = response.into_body().data().await {
            tracing::debug!("RPC Body:\n{:}", String::from_utf8_lossy(&body));
        }

        // Delete plugin file.
        if let Err(e) = tokio::fs::remove_file(self.metrics_path).await {
            tracing::warn!("Unable to remove plugin file: {e}");
        }
    }
}
