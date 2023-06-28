use std::path::{Path, PathBuf};

use tokio::{fs::OpenOptions, io::AsyncWriteExt};
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
fn values_to_raw(values: &[DataSourceValue]) -> Box<[[u8; 8]]> {
    values
        .iter()
        .map(|value| match value {
            DataSourceValue::Int64(n) => (*n).to_be_bytes(),
            DataSourceValue::Float(f) => (*f).to_be_bytes(),
            DataSourceValue::Undefined => [0; 8],
        })
        .collect()
}

impl RrddPlugin {
    pub async fn new(
        uid: &'_ str,
        metadata: RrddMetadata,
        initial_values: Option<&[DataSourceValue]>,
    ) -> anyhow::Result<Self> {
        let (header, metadata_str) = Self::generate_initial_header(metadata, initial_values)?;

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
        self.header.update_values(&values_to_raw(new_values))?;
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
        let (header, metadata_str) =
            Self::generate_initial_header(metadata.clone(), initial_values)?;

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

    fn generate_initial_header(
        metadata: RrddMetadata,
        initial_values: Option<&[DataSourceValue]>,
    ) -> anyhow::Result<(RrddMessageHeader, Box<str>)> {
        let raw_values = if let Some(init) = initial_values {
            values_to_raw(init)
        } else {
            vec![[0; 8]; metadata.datasources.len()].into_boxed_slice()
        };

        Ok(RrddMessageHeader::generate(&raw_values, metadata))
    }
}
