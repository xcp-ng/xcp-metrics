use std::time::Duration;
use tokio::time;

use xcp_metrics_common::rrdd::{
    protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
    protocol_v2::{indexmap::indexmap, RrddMetadata},
};
use xcp_metrics_plugin_common::protocol_v2::RrddPlugin;

#[tokio::main]
async fn main() {
    let metadata = RrddMetadata {
        datasources: indexmap! {
            "some_imaginary_cpu_usage".into() => DataSourceMetadata {
                description: "Some value".into(),
                units: "(fraction)".into(),
                ds_type: DataSourceType::Absolute,
                value: DataSourceValue::Float(50.0),
                min: 0.0,
                max: 100.0,
                owner: DataSourceOwner::Host,
                default: true,
            },
            "another thing".into() => DataSourceMetadata {
                description: "Another Value".into(),
                units: "a-b-c".into(),
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

    let mut plugin = RrddPlugin::new("xcp-metrics-plugin-characters", metadata, Some(&values))
        .await
        .unwrap();

    // Expose protocol v2
    loop {
        // Update sources
        plugin.update_values(&values).await.unwrap();
        time::sleep(Duration::from_secs(1)).await;
    }
}
