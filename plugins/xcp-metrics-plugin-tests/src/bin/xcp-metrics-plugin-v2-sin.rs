use std::time::Duration;
use tokio::time::{self, Instant};

use xcp_metrics_common::rrdd::{
    protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
    protocol_v2::{indexmap::indexmap, RrddMetadata},
};
use xcp_metrics_plugin_common::protocol_v2::RrddPlugin;

#[tokio::main]
async fn main() {
    let metadata = RrddMetadata {
        datasources: indexmap! {
            "sin".into() => DataSourceMetadata {
                description: "A variation over time".into(),
                units: "number".into(),
                ds_type: DataSourceType::Gauge,
                value: DataSourceValue::Float(0.0),
                min: -1.0,
                max: 1.0,
                owner: DataSourceOwner::Host,
                default: true,
            }
        },
    };

    let mut values = [DataSourceValue::Float(0.0)];
    let time = Instant::now();

    let mut plugin = RrddPlugin::new("xcp-metrics-plugin-sin", metadata, Some(&values), None)
        .await
        .unwrap();

    // Expose protocol v2
    loop {
        // Update sources
        values[0] = DataSourceValue::Float(f64::sin(time.elapsed().as_secs_f64()));

        plugin.update_values(&values).await.unwrap();
        time::sleep(Duration::from_secs(1)).await;
    }
}
