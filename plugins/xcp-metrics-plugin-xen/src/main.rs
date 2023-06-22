use std::time::Duration;
use tokio::time;

use xcp_metrics_common::rrdd::{
    protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
    protocol_v2::{indexmap::indexmap, RrddMetadata},
};
use xcp_metrics_plugin_common::RrddPlugin;

#[tokio::main]
async fn main() {
    let datasources = indexmap! {
        "nice_metrics".into() => DataSourceMetadata {
            description: "something".into(),
            units: "unit_test".into(),
            ds_type: DataSourceType::Gauge,
            value: DataSourceValue::Int64(1),
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
            owner: DataSourceOwner::Host,
            default: true,
        },
        "thats_great".into() => DataSourceMetadata {
            description: "something_else".into(),
            units: "unit test".into(),
            ds_type: DataSourceType::Gauge,
            value: DataSourceValue::Float(1.0),
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
            owner: DataSourceOwner::Host,
            default: true,
        },
    };

    let metadata = RrddMetadata { datasources };

    let values = [
        DataSourceValue::Int64(42),
        DataSourceValue::Float(3.14159265),
    ];

    let mut plugin = RrddPlugin::new("xcp-metrics-plugin-xen", metadata, Some(&values))
        .await
        .unwrap();

    // Expose protocol v2
    loop {
        plugin.update_values(&values).await.unwrap();
        time::sleep(Duration::from_secs(1)).await;
    }
}
