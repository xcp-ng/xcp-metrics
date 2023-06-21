use std::{f32::INFINITY, fs::OpenOptions, io::Write, thread, time::Duration};

use xcp_metrics_common::{
    rrdd::{
        protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
        protocol_v2::{RrddMessageHeader, RrddMetadata},
    },
    xapi,
    xmlrpc::PluginLocalRegister,
};

#[tokio::main]
async fn main() {
    let request = PluginLocalRegister {
        info: "Five_Seconds".into(),
        protocol: "V2".into(),
        uid: "xcp-metrics-plugin-xen".into(),
    };

    xapi::send_xmlrpc_at("xcp-rrdd", "POST", &request, "xcp-metrics")
        .await
        .unwrap();

    let mut options = OpenOptions::new();
    options.truncate(false);
    options.write(true);

    let metadata = RrddMetadata {
        datasources: [
            DataSourceMetadata {
                description: "a".into(),
                units: "unit test".into(),
                ds_type: DataSourceType::Absolute,
                value: DataSourceValue::Int64(0),
                min: 0.0,
                max: INFINITY,
                owner: DataSourceOwner::Host,
                default: true,
            },
            DataSourceMetadata {
                description: "b".into(),
                units: "unit test".into(),
                ds_type: DataSourceType::Absolute,
                value: DataSourceValue::Int64(0),
                min: 0.0,
                max: INFINITY,
                owner: DataSourceOwner::Host,
                default: true,
            },
        ]
        .into(),
    };

    let values = [[1u8; 8], [2u8; 8]];

    let (rrdd_header, metadata) = RrddMessageHeader::generate(&values, metadata);

    {
        let mut file = options.open("/dev/shm/xcp-metrics-plugin-xen").unwrap();

        rrdd_header.write(&mut file).unwrap();
        file.write_all(metadata.as_bytes()).unwrap();
    }

    // Expose protocol v2
    loop {
        match options.open("/dev/shm/xcp-metrics-plugin-xen") {
            Ok(mut file) => {
                rrdd_header.write(&mut file).unwrap();
            }
            Err(e) => println!("{e:?}"),
        }

        thread::sleep(Duration::from_secs(5));
    }
}
