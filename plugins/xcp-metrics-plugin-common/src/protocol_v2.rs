use std::path::{Path, PathBuf};

use tokio::{
    fs::{create_dir_all, OpenOptions},
    io::AsyncWriteExt,
};
use xcp_metrics_common::{
    rpc::methods::{PluginLocalDeregister, PluginLocalRegister},
    rrdd::{
        protocol_common::DataSourceValue,
        protocol_v2::{values_to_raw, RrddMessageHeader, RrddMetadata},
    },
    xapi::{self, hyper::body::HttpBody, METRICS_SHM_PATH},
};

pub struct RrddPlugin {
    uid: Box<str>,
    header: RrddMessageHeader,
    metrics_path: PathBuf,
    target_daemon: Box<str>,
}

const DEFAULT_DAEMON: &str = "xcp-metrics";

impl RrddPlugin {
    /// Create and register a new plugin.
    pub async fn new(
        uid: &'_ str,
        metadata: RrddMetadata,
        initial_values: Option<&[DataSourceValue]>,
        target_daemon: Option<&str>,
    ) -> anyhow::Result<Self> {
        let (header, metadata_str) = Self::generate_initial_header(metadata, initial_values);

        let plugin = Self {
            uid: uid.into(),
            header,
            metrics_path: Path::new(METRICS_SHM_PATH).join(uid),
            target_daemon: target_daemon.unwrap_or(DEFAULT_DAEMON).into(),
        };

        plugin.reset_file(Some(&metadata_str)).await?;
        plugin.advertise_plugin().await?;

        Ok(plugin)
    }

    /// Push new values to the shared file.
    ///
    /// # Condition
    /// The length of `new_values` must match the latest sent metadata (either by [Self::new] or [Self::reset_metadata]).
    pub async fn update_values(&mut self, new_values: &[DataSourceValue]) -> anyhow::Result<()> {
        self.header.update_values(&values_to_raw(new_values))?;
        self.reset_file(None).await
    }

    /// Advertise the existence of the plugin to the main daemon.
    pub async fn advertise_plugin(&self) -> anyhow::Result<()> {
        let request = PluginLocalRegister {
            info: "Five_Seconds".into(),
            protocol: "V2".into(),
            uid: (*self.uid).into(),
        };

        let response = xapi::send_xmlrpc_at(
            &self.target_daemon,
            "POST",
            &request,
            &self.uid, /* use uid as user-agent */
        )
        .await
        .map_err(|e| anyhow::anyhow!("Can't reach '{}' daemon ({e})", self.target_daemon))?;

        tracing::debug!("RPC Response: {response:?}");
        if let Some(Ok(body)) = response.into_body().data().await {
            tracing::debug!("RPC Body:\n{:}", String::from_utf8_lossy(&body));
        }

        Ok(())
    }

    /// Replace the metadata of the shared file.
    ///
    /// # Condition
    /// The length of `initial_values` must match the `metadata`.
    pub async fn reset_metadata(
        &mut self,
        metadata: RrddMetadata,
        initial_values: Option<&[DataSourceValue]>,
    ) -> anyhow::Result<()> {
        let (header, metadata_str) =
            Self::generate_initial_header(metadata.clone(), initial_values);

        self.header = header;
        self.reset_file(Some(&metadata_str)).await
    }

    async fn reset_file(&self, raw_metadata: Option<&str>) -> anyhow::Result<()> {
        // Create directory if doesn't exist.
        create_dir_all(METRICS_SHM_PATH).await?;

        let mut options = OpenOptions::new();
        options.create(true);
        options.truncate(false);
        options.write(true);

        let mut file = options.open(&self.metrics_path).await?;

        let mut header_buffer = vec![];
        self.header.write(&mut header_buffer)?;
        file.write_all(&header_buffer).await?;

        if let Some(raw_metadata) = raw_metadata {
            file.write_all(raw_metadata.as_bytes()).await?;
        }

        Ok(())
    }

    fn generate_initial_header(
        metadata: RrddMetadata,
        initial_values: Option<&[DataSourceValue]>,
    ) -> (RrddMessageHeader, Box<str>) {
        let raw_values = if let Some(init) = initial_values {
            values_to_raw(init)
        } else {
            vec![[0; 8]; metadata.datasources.len()].into_boxed_slice()
        };

        RrddMessageHeader::generate(&raw_values, metadata)
    }

    /// Deregister the plugin from the daemon.
    pub async fn deregister_plugin(self) {
        tracing::info!("Deregistering {}...", &self.uid);

        // Unregister plugin
        let request = PluginLocalDeregister {
            uid: self.uid.to_string(),
        };

        match xapi::send_xmlrpc_at(
            &self.target_daemon,
            "POST",
            &request,
            &self.uid, /* use uid as user-agent */
        )
        .await
        {
            Ok(response) => {
                tracing::debug!("RPC Response: {response:?}");
                if let Some(Ok(body)) = response.into_body().data().await {
                    tracing::debug!("RPC Body:\n{:}", String::from_utf8_lossy(&body));
                }
            }
            Err(e) => {
                tracing::error!("Unable to unregister plugin ({e})")
            }
        }

        // Delete plugin file.
        if let Err(e) = tokio::fs::remove_file(self.metrics_path).await {
            tracing::warn!("Unable to remove plugin file: {e}");
        }
    }
}
