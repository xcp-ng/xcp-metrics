//mod rpc;
//mod rrdd;
//mod xapi;

pub mod hub;

use std::{io::Read, path::Path, time::Duration};

use tokio::time;
use xcp_metrics_common::rrdd::{
    protocol_common::DataSourceValue,
    protocol_v2::{RrddMessageHeader, RrddMetadata, RrddMetadataRaw},
};

const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

#[derive(Debug, Clone)]
pub struct PluginData {
    metadata: RrddMetadata,
    values: Box<[DataSourceValue]>,
    metadata_checksum: u32,
}

fn collect_plugin_metrics(name: &str, state: &mut Option<PluginData>) -> anyhow::Result<()> {
    let path = Path::new(METRICS_SHM_PATH).join(name);

    let mut file = std::fs::File::open(path)?;
    let header = RrddMessageHeader::parse_from(&mut file);

    println!("{name}: Readed {header:?}");

    if let Ok(header) = header {
        // Get the most up to date PluginData.
        let mut data = match state.as_ref() {
            /* matching checksums, no need to update metadata */
            Some(
                data @ &PluginData {
                    metadata_checksum, ..
                },
            ) if metadata_checksum == header.metadata_checksum => (*data).clone(),

            /* Regenerate data */
            _ => {
                println!("{name}: Update metadata");

                // Read metadata
                let mut metadata_string = vec![0u8; header.metadata_length as usize];
                file.read_exact(&mut metadata_string)?;
                let metadata: RrddMetadata =
                    serde_json::from_slice::<RrddMetadataRaw>(&metadata_string)?.try_into()?;

                PluginData {
                    values: vec![DataSourceValue::Undefined; metadata.datasources.len()]
                        .into_boxed_slice(),
                    metadata,
                    metadata_checksum: header.metadata_checksum,
                }
            }
        };

        // Update data value slice using raw values in header along with metadata.
        data.values
            .iter_mut()
            .zip(data.metadata.datasources.values())
            .zip(header.values.iter())
            .for_each(|((dest, meta), raw)| {
                *dest = match meta.value {
                    DataSourceValue::Int64(_) => DataSourceValue::Int64(i64::from_be_bytes(*raw)),
                    DataSourceValue::Float(_) => DataSourceValue::Float(f64::from_be_bytes(*raw)),
                    DataSourceValue::Undefined => DataSourceValue::Undefined,
                }
            });

        state.replace(data);
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // Redesign it ?
    let mut plugin_data = None;
    let plugin_name = "xcp-metrics-plugin-xen";

    loop {
        println!(
            "{:?}",
            collect_plugin_metrics(plugin_name, &mut plugin_data)
        );
        println!("{plugin_data:?}");

        time::sleep(Duration::from_secs(5)).await;
    }
}
