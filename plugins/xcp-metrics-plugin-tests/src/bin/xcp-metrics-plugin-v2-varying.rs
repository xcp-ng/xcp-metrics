use std::io;

use xcp_metrics_common::rrdd::{
    protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
    protocol_v2::{
        indexmap::{indexmap, IndexMap},
        RrddMetadata,
    },
};
use xcp_metrics_plugin_common::protocol_v2::RrddPlugin;

#[tokio::main]
async fn main() {
    // Expose protocol v2
    let metadata = RrddMetadata {
        datasources: indexmap!(),
    };
    let values = [];

    let mut plugin = RrddPlugin::new("xcp-metrics-plugin-varying", metadata, Some(&values))
        .await
        .unwrap();

    loop {
        // Generate some random datasources
        let datasources: IndexMap<Box<str>, DataSourceMetadata> = (0..fastrand::usize(..10))
            .map(|i| {
                (
                    format!("m{i}_{}", uuid::Uuid::new_v4().as_simple()).into_boxed_str(),
                    DataSourceMetadata {
                        description: format!("Test metric {i}").into(),
                        units: "".into(),
                        ds_type: DataSourceType::Gauge,
                        value: DataSourceValue::Int64(0),
                        min: 0.0,
                        max: f32::INFINITY,
                        owner: DataSourceOwner::Host,
                        default: true,
                    },
                )
            })
            .collect();

        let values: Vec<DataSourceValue> = datasources
            .iter()
            .map(|_| DataSourceValue::Int64(fastrand::i64(0..)))
            .collect();

        let metadata = RrddMetadata { datasources };

        println!("Values: {values:#?}");
        println!("Metadata: {metadata:#?}");

        // Update sources
        plugin
            .reset_metadata(metadata, Some(&values))
            .await
            .unwrap();

        println!("Press a key to reset values and metadata");
        io::stdin().read_line(&mut String::default()).unwrap();
    }
}
