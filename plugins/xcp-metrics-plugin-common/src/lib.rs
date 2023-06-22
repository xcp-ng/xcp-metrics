use std::path::{Path, PathBuf};

use tokio::{
    fs::{self, OpenOptions},
    io::AsyncWriteExt,
};
use xcp_metrics_common::{
    rrdd::{
        protocol_common::DataSourceValue,
        protocol_v2::{RrddMessageHeader, RrddMetadata},
    },
    xapi::{self, hyper::body::HttpBody},
    xmlrpc::PluginLocalRegister,
};

pub struct RrddPlugin {
    uid: Box<str>,
    header: RrddMessageHeader,
    metrics_path: PathBuf,
}

const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

/// Update `header` values with ones provided by `values`.
fn update_header_value(
    header: &mut RrddMessageHeader,
    values: &[DataSourceValue],
) -> anyhow::Result<()> {
    if header.values.len() != values.len() {
        anyhow::bail!("Header value count and values count doesn't match");
    }

    header
        .values
        .iter_mut()
        .zip(values.iter())
        .for_each(|(buffer, value)| {
            *buffer = match value {
                DataSourceValue::Int64(n) => (*n).to_be_bytes(),
                DataSourceValue::Float(f) => (*f).to_be_bytes(),
                DataSourceValue::Undefined => [0; 8],
            }
        });

    Ok(())
}

impl RrddPlugin {
    pub async fn new(
        uid: &'_ str,
        metadata: RrddMetadata,
        initial_values: Option<&[DataSourceValue]>,
    ) -> anyhow::Result<Self> {
        let (header, metadata_str) = Self::regenerate_metadata(metadata, initial_values)?;

        let plugin = Self {
            uid: uid.into(),
            header,
            metrics_path: Path::new(METRICS_SHM_PATH).join(uid),
        };

        plugin.reset_file(Some(&metadata_str)).await?;
        plugin.advertise_plugin().await?;

        Ok(plugin)
    }

    pub async fn update_values(&mut self, new_values: &[DataSourceValue]) -> anyhow::Result<()> {
        update_header_value(&mut self.header, new_values)?;

        self.reset_file(None).await
    }

    pub async fn advertise_plugin(&self) -> anyhow::Result<()> {
        let request = PluginLocalRegister {
            info: "Five_Seconds".into(),
            protocol: "V2".into(),
            uid: (*self.uid).into(),
        };

        let response = xapi::send_xmlrpc_at(
            "xcp-rrdd", "POST", &request, &self.uid, /* use uid as user-agent */
        )
        .await?;

        println!("RPC Response: {response:?}");
        if let Some(Ok(body)) = response.into_body().data().await {
            println!("RPC Body:\n{:}", String::from_utf8_lossy(&body));
        }

        Ok(())
    }

    pub async fn reset_metadata(
        &mut self,
        metadata: RrddMetadata,
        initial_values: Option<&[DataSourceValue]>,
    ) -> anyhow::Result<()> {
        let (header, metadata_str) = Self::regenerate_metadata(metadata, initial_values)?;

        self.header = header;
        self.reset_file(Some(&metadata_str)).await
    }

    async fn reset_file(&self, raw_metadata: Option<&str>) -> anyhow::Result<()> {
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

    fn regenerate_metadata(
        metadata: RrddMetadata,
        initial_values: Option<&[DataSourceValue]>,
    ) -> anyhow::Result<(RrddMessageHeader, Box<str>)> {
        let values = initial_values.map(|v| v.to_vec()).unwrap_or_else(|| {
            metadata
                .datasources
                .iter()
                .map(|(_, ds)| ds.value)
                .collect()
        });

        let raw_values = vec![[0; 8]; values.len()];
        let (mut header, metadata_str) = RrddMessageHeader::generate(&raw_values, metadata);

        update_header_value(&mut header, &values)?;

        Ok((header, metadata_str))
    }
}
