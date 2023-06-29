use std::{f64, time::Duration};
use tokio::time;

use xcp_metrics_common::rrdd::{
    protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
    protocol_v2::{indexmap::indexmap, RrddMetadata},
};
use xcp_metrics_plugin_common::RrddPlugin;

#[tokio::main]
async fn main() {
    let xen = xenctrl::XenControl::default().unwrap();

    for domid in 0.. {
        match xen.domain_getinfo(domid) {
            Ok(Some(dominfo)) => {
                println!("{dominfo:#?}");

                for vcpuid in 0..dominfo.max_vcpu_id {
                    match xen.vcpu_getinfo(domid, vcpuid) {
                        Ok(vcpuinfo) => println!("{vcpuinfo:#?}"),
                        Err(e) => {
                            println!("{e:?}");
                            break;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("{e}");
                break;
            }
            _ => break,
        }
    }

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

    let mut counter = 1;
    let mut values = vec![
        DataSourceValue::Int64(42),
        DataSourceValue::Float(f64::consts::PI),
    ];

    let mut plugin = RrddPlugin::new("xcp-metrics-plugin-xen", metadata, Some(&values))
        .await
        .unwrap();

    // Expose protocol v2
    loop {
        counter += 1;
        values[0] = DataSourceValue::Int64(counter);

        plugin.update_values(&values).await.unwrap();
        time::sleep(Duration::from_secs(1)).await;
    }
}
