//mod rpc;
//mod rrdd;
//mod xapi;

use std::{path::Path, sync::Arc};

use dashmap::DashMap;
use tokio::{sync::RwLock, task::JoinSet};
use xcp_metrics_common::rrdd::{
    protocol_common::DataSourceValue,
    protocol_v2::{RrddMessageHeader, RrddMetadata, RrddMetadataRaw},
};

const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

#[derive(Clone)]
pub struct PluginData {
    metadata: RrddMetadata,
    values: Box<[DataSourceValue]>,
    metadata_checksum: u32,
}

fn collect_plugin_metrics(
    name: &str,
    state: Arc<RwLock<Option<PluginData>>>,
) -> anyhow::Result<()> {
    let path = Path::new(METRICS_SHM_PATH).join(name);

    let mut file = std::fs::File::open(path)?;
    let header = RrddMessageHeader::parse_from(&mut file);

    println!("{name}: Readed {header:?}");

    if let Ok(header) = header {
        // Check metadata checksum
        let data = match state.blocking_read().as_ref() {
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
                let metadata_string = vec![0u8; header.metadata_length as usize];
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
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    // Redesign it ?
    let plugins: DashMap<&str, Arc<RwLock<Option<PluginData>>>> = DashMap::default();

    plugins.insert("xcp-metrics-plugin-xen", Default::default());

    loop {
        let mut join_set = JoinSet::new();

        plugins.clone().into_iter().for_each(|(name, state)| {
            join_set.spawn_blocking(|| collect_plugin_metrics(name, state));
        });

        while let Some(_) = join_set.join_next().await {}
    }
}
