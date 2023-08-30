use std::{io::Read, iter};

use xcp_metrics_common::rrdd::{
    protocol_common::DataSourceValue,
    protocol_v2::{self, values_to_raw, RrddMessageHeader, RrddMetadata},
};
use xcp_metrics_plugin_common::{
    bridge::v3_to_v2::BridgeToV2,
    plugin::XcpPlugin,
    xenstore::{mock::MockXs, write::XsWrite, xs::XBTransaction},
};

use crate::{SqueezedInfo, SqueezedPlugin};

#[test]
fn no_vm() {
    // No virtual machine : all 0
    let xs = MockXs::default();

    xs.write(XBTransaction::Null, "/local/domain", "").unwrap();

    assert_eq!(
        SqueezedInfo::get(&xs).unwrap(),
        SqueezedInfo {
            reclaimed: 0,
            reclaimed_max: 0
        }
    );
}

#[test]
fn single_vm() {
    let xs = MockXs::default();

    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/target",
        "123456",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/dynamic-min",
        "0",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/dynamic-max",
        "654321",
    )
    .unwrap();

    assert_eq!(
        SqueezedInfo::get(&xs).unwrap(),
        SqueezedInfo {
            reclaimed: 530865,
            reclaimed_max: 123456
        }
    );
}

#[test]
fn multiple_vm() {
    let xs = MockXs::default();

    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/target",
        "123456",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/dynamic-min",
        "0",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/0/memory/dynamic-max",
        "654321",
    )
    .unwrap();

    // Consider missing domain 1.

    xs.write(
        XBTransaction::Null,
        "/local/domain/2/memory/target",
        "111111",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/2/memory/dynamic-min",
        "0",
    )
    .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/2/memory/dynamic-max",
        "999999",
    )
    .unwrap();

    assert_eq!(
        SqueezedInfo::get(&xs).unwrap(),
        SqueezedInfo {
            reclaimed: 530865 + 888888,
            reclaimed_max: 123456 + 111111
        }
    );
}

#[test]
fn test_export() {
    let xs = MockXs::default();

    // /local/domain is needed for squeezed plugin to work properly
    // Having this entry missing will create warnings and no families at the end.
    xs.write(XBTransaction::Null, "/local/domain", "").unwrap();

    xs.write(XBTransaction::Null, "/local/domain/0", "")
        .unwrap();
    xs.write(
        XBTransaction::Null,
        "/local/domain/2/dynamic-max",
        "1048576",
    )
    .unwrap();
    xs.write(XBTransaction::Null, "/local/domain/2/dynamic-min", "524288")
        .unwrap();
    xs.write(XBTransaction::Null, "/local/domain/2/target", "1048576")
        .unwrap();

    let mut plugin = SqueezedPlugin { xs };

    let mut bridge = BridgeToV2::new();
    bridge.update(plugin.generate_metrics().into());

    let metadata = bridge.get_metadata();
    let data = bridge.get_data();

    // Generate a rrdd header
    let (header, _) = RrddMessageHeader::generate(&values_to_raw(&data), metadata.clone());

    // Read a reference one generated with rrdp-squeezed
    let reference_payload = &mut include_bytes!("../tests/xcp-rrdd-squeezed").as_slice();
    let reference_header = protocol_v2::RrddMessageHeader::parse_from(reference_payload).unwrap();

    let metadata_part = Read::take(reference_payload, reference_header.metadata_length as u64);
    let mut reference_metadata: RrddMetadata = serde_json::from_reader(metadata_part).unwrap();

    // Normalize reference datasource values (they are not used in protocol v2).
    reference_metadata
        .datasources
        .iter_mut()
        .for_each(|(_, ds)| ds.value = DataSourceValue::Int64(0));

    // Check if metadata matches
    assert_eq!(metadata, &reference_metadata);

    // Check if values matches.
    // Order may not be the same between xcp-rrdd-squeezed and this plugin (due to some hashmap randomness).
    // We need to compare each values, regardless of the order.
    iter::zip(
        reference_metadata.datasources.iter(),
        reference_header.values.iter(),
    )
    .for_each(|((reference_name, _), reference_raw_value)| {
        // Get matching raw value in generated data.
        let value = iter::zip(metadata.datasources.iter(), header.values.iter())
            // Get matching value
            .find(|((name, _), _)| reference_name.as_ref() == name.as_ref())
            // Convert DataSourceValue into raw bytes.
            .map(|(_, value)| value)
            .expect(&format!("Missing name {reference_name}"));

        assert_eq!(
            value, reference_raw_value,
            "{reference_name} differs: {value:?} != {reference_raw_value:?}"
        );
    });
}
