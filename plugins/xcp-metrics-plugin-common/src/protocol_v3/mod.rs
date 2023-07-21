//! xcp-metrics plugin protocol v3 framework.
pub mod utils;

use std::path::{Path, PathBuf};

use tokio::fs::OpenOptions;
use xcp_metrics_common::{
    metrics::MetricSet,
    protocol_v3,
    rpc::methods::{PluginLocalDeregister, PluginLocalRegister},
    xapi::{self, hyper::body::HttpBody, METRICS_SHM_PATH},
};

pub struct MetricsPlugin {
    uid: Box<str>,
    metrics_path: PathBuf,
}

impl MetricsPlugin {
    /// Create and register a new plugin.
    pub async fn new(uid: &'_ str, metrics: MetricSet) -> anyhow::Result<Self> {
        let plugin = Self {
            uid: uid.into(),
            metrics_path: Path::new(METRICS_SHM_PATH).join(uid),
        };

        plugin.update(metrics).await?;

        Ok(plugin)
    }

    pub async fn update(&self, metrics: MetricSet) -> anyhow::Result<()> {
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
            "xcp-metrics", "POST", &request, &self.uid, /* use uid as user-agent */
        )
        .await?;

        println!("RPC Response: {response:?}");
        if let Some(Ok(body)) = response.into_body().data().await {
            println!("RPC Body:\n{:}", String::from_utf8_lossy(&body));
        }

        Ok(())
    }

    /// Deregister the plugin from the daemon.
    pub async fn deregister_plugin(self) {
        println!("Deregistering {}...", &self.uid);

        // Unregister plugin
        let request = PluginLocalDeregister {
            uid: self.uid.to_string(),
        };

        let response = xapi::send_xmlrpc_at(
            "xcp-rrdd", "POST", &request, &self.uid, /* use uid as user-agent */
        )
        .await
        .unwrap();

        println!("RPC Response: {response:?}");
        if let Some(Ok(body)) = response.into_body().data().await {
            println!("RPC Body:\n{:}", String::from_utf8_lossy(&body));
        }

        // Delete plugin file.
        if let Err(e) = tokio::fs::remove_file(self.metrics_path).await {
            println!("Unable to remove plugin file: {e}");
        }
    }
}
