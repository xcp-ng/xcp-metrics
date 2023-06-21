use std::{
    collections::BTreeMap, f32::INFINITY, fs::OpenOptions, io::Write, thread, time::Duration,
};

use xcp_metrics_common::{
    rrdd::{
        protocol_common::{DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue},
        protocol_v2::{RrddMessageHeader, RrddMetadata},
    },
    xapi::{self, hyper::body::HttpBody},
    xmlrpc::PluginLocalRegister,
};

#[tokio::main]
async fn main() {
    let request = PluginLocalRegister {
        info: "Five_Seconds".into(),
        protocol: "V2".into(),
        uid: "xcp-metrics-plugin-xen".into(),
    };

    let response = xapi::send_xmlrpc_at("xcp-rrdd", "POST", &request, "xcp-metrics-plugin-xen").await;

    println!("{:#?}", response);

    if let Ok(mut body) = response.map(|r: xapi::hyper::Response<xapi::hyper::Body>| r.into_body())
    {
        if let Some(Ok(content)) = body.data().await {
            println!("{}", String::from_utf8_lossy(&content.to_vec()));
        }
    }

    let datasources = BTreeMap::from([
        (
            "nice_metrics".into(),
            DataSourceMetadata {
                description: "something".into(),
                units: "unit_test".into(),
                ds_type: DataSourceType::Gauge,
                value: DataSourceValue::Int64(42),
                min: 0.0,
                max: 100000.0,
                owner: DataSourceOwner::Host,
                default: true,
            },
        ),
        (
            "thats_great".into(),
            DataSourceMetadata {
                description: "something_else".into(),
                units: "unit test".into(),
                ds_type: DataSourceType::Gauge,
                value: DataSourceValue::Int64(42),
                min: 0.0,
                max: 100000.0,
                owner: DataSourceOwner::Host,
                default: true,
            },
        ),
    ]);

    let metadata = RrddMetadata { datasources };

    let values = [[1u8; 8], [2u8; 8]];

    let (rrdd_header, metadata) = RrddMessageHeader::generate(&values, metadata);

    let mut options = OpenOptions::new();
    options.truncate(false);
    options.write(true);
    options.create(true);

    {
        let mut file = options
            .open("/dev/shm/metrics/xcp-metrics-plugin-xen")
            .unwrap();

        rrdd_header.write(&mut file).unwrap();
        file.write_all(metadata.as_bytes()).unwrap();
    }

    // Expose protocol v2
    loop {
        match options.open("/dev/shm/metrics/xcp-metrics-plugin-xen") {
            Ok(mut file) => {
                rrdd_header.write(&mut file).unwrap();
            }
            Err(e) => println!("{e:?}"),
        }

        thread::sleep(Duration::from_secs(5));
    }
}
