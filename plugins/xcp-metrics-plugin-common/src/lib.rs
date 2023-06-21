use std::{
    io::Write,
    path::{Path, PathBuf},
};
use xcp_metrics_common::{
    rrdd::{
        protocol_common::DataSourceValue,
        protocol_v2::{RrddMessageHeader, RrddMetadata},
    },
    xapi,
    xmlrpc::PluginLocalRegister,
};

pub struct RrddPlugin {
    uid: Box<str>,
    header: RrddMessageHeader,
    values: Vec<DataSourceValue>,
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
        let values = initial_values.map(|v| v.to_vec()).unwrap_or_else(|| {
            metadata
                .datasources
                .iter()
                .map(|(_, ds)| ds.value)
                .collect()
        });

        let raw_values = vec![[0; 8]; values.len()];
        let (mut header, metadata) = RrddMessageHeader::generate(&raw_values, metadata);

        update_header_value(&mut header, &values);

        let plugin = Self {
            uid: uid.into(),
            header,
            values,
            metrics_path: Path::new(METRICS_SHM_PATH).join(uid),
        };

        plugin.advertise_plugin().await?;

        Ok(plugin)
    }

    pub fn update_values(&mut self, new_values: &[DataSourceValue]) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn advertise_plugin(&self) -> anyhow::Result<()> {
        let request = PluginLocalRegister {
            info: "Five_Seconds".into(),
            protocol: "V2".into(),
            uid: self.uid.into(),
        };

        let response = xapi::send_xmlrpc_at(
            "xcp-rrdd", "POST", &request, &self.uid, /* use uid as user-agent */
        )
        .await?;

        if let Ok(mut body) =
            response.map(|r: xapi::hyper::Response<xapi::hyper::Body>| r.into_body())
        {
            if let Some(Ok(content)) = body.data().await {
                println!("{}", String::from_utf8_lossy(&content.to_vec()));
            }
        }

        Ok(())
    }

    async fn reset_file(&self, raw_metadata: &str) -> anyhow::Result<()> {
        let mut file = std::fs::File::create(self.metrics_path)?;

        self.header.write(&mut file)?;
        file.write_all(raw_metadata.as_bytes())?;

        Ok(())
    }
}
