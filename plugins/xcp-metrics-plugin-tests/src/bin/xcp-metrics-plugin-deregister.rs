use std::time::Duration;
use tokio::time;

use xcp_metrics_common::rrdd::{
    protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
    protocol_v2::{indexmap::indexmap, RrddMetadata},
};
use xcp_metrics_plugin_common::RrddPlugin;

#[tokio::main]
async fn main() {
    let metadata = RrddMetadata {
        datasources: indexmap! {
            "life".into() => DataSourceMetadata {
                description: "Answer to the Ultimate Question of Life, The Universe, and Everything".into(),
                units: "everything".into(),
                ds_type: DataSourceType::Absolute,
                value: DataSourceValue::Int64(42),
                min: 42.0,
                max: 42.0,
                owner: DataSourceOwner::Host,
                default: true,
            },
            "pi".into() => DataSourceMetadata {
                description: "The PI number".into(),
                units: "rad".into(),
                ds_type: DataSourceType::Absolute,
                value: DataSourceValue::Float(std::f64::consts::PI),
                min: 3.0,
                max: 4.0,
                owner: DataSourceOwner::Host,
                default: true,
            }
        },
    };

    let values = [
        DataSourceValue::Int64(42),
        DataSourceValue::Float(std::f64::consts::PI),
    ];

    let mut plugin = RrddPlugin::new("xcp-metrics-plugin-deregister", metadata, Some(&values))
        .await
        .unwrap();

    // Expose protocol v2
    // Update sources
    plugin.update_values(&values).await.unwrap();

    println!("Wait 10 seconds before plugin gets terminated...");
    time::sleep(Duration::from_secs(10)).await;

    plugin.deregister_plugin().await
}
