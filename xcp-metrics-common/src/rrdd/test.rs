use std::{
    io::{Read, Write},
    time::{Duration, SystemTime},
};

use indexmap::indexmap;

use crate::rrdd::protocol_v2::{RrddMessageHeader, RrddMetadata, RrddMetadataRaw};

use super::{protocol_common::DataSourceValue, protocol_v2::values_to_raw};

/// Check if metadata stays the same after being encoded then decoded.
#[test]
fn metadata_invariance() {
    let metadata = RrddMetadata {
        datasources: indexmap! {
          "A".into() => Default::default(),
          "B".into() => Default::default(),
        },
    };

    let metadata_raw: RrddMetadataRaw = metadata.clone().into();

    let metadata_reparsed: RrddMetadata = metadata_raw.try_into().unwrap();

    assert_eq!(metadata, metadata_reparsed);
}

fn generate_test_rrdd() -> (RrddMetadata, RrddMessageHeader, Vec<u8>) {
    let metadata = RrddMetadata {
        datasources: indexmap! {
          "A".into() => Default::default()
        },
    };
    let values = [u64::to_be_bytes(42)];

    let (mut header, metadata_str) = RrddMessageHeader::generate(&values, metadata.clone());

    // Remove subsec ns precision (protocol v2 only provide seconds accuracy)
    let ns_diff = header
        .timestamp
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    header.timestamp -= Duration::from_nanos(ns_diff as u64);

    let mut buffer = vec![];
    header.write(&mut buffer).unwrap();
    buffer.write_all(metadata_str.as_bytes()).unwrap();
    (metadata, header, buffer)
}

/// Check if protocol v2 header stays the same after being encoded then decoded.
/// NOTE: Timestamp precision is not kept.
#[test]
fn invariance() {
    let (metadata, header, buffer) = generate_test_rrdd();

    let mut reader = buffer.as_slice();
    let header_readed = RrddMessageHeader::parse_from(&mut reader).unwrap();
    assert_eq!(header, header_readed);

    let mut metadata_buffer = vec![0u8; header_readed.metadata_length as usize];
    reader.read_exact(&mut metadata_buffer).unwrap();
    let metadata_raw_readed: RrddMetadataRaw = serde_json::from_slice(&metadata_buffer).unwrap();
    let metadata_readed = metadata_raw_readed.try_into().unwrap();

    assert_eq!(metadata, metadata_readed);
}

#[test]
fn invariance_async() {
    smol::block_on(async {
        let (metadata, header, buffer) = generate_test_rrdd();

        let mut reader = buffer.as_slice();
        let header_readed = RrddMessageHeader::parse_async(&mut reader).await.unwrap();
        assert_eq!(header, header_readed);

        let mut metadata_buffer = vec![0u8; header_readed.metadata_length as usize];
        reader.read_exact(&mut metadata_buffer).unwrap();
        let metadata_raw_readed: RrddMetadataRaw =
            serde_json::from_slice(&metadata_buffer).unwrap();
        let metadata_readed = metadata_raw_readed.try_into().unwrap();

        assert_eq!(metadata, metadata_readed);
    });
}

#[test]
fn test_values_to_raw() {
    let values = [
        DataSourceValue::Float(123.0),
        DataSourceValue::Int64(0),
        DataSourceValue::Int64(1),
        DataSourceValue::Undefined,
    ];

    let raw = values_to_raw(&values);

    assert_eq!(
        raw.as_ref(),
        &[123.0f64.to_be_bytes(), [0; 8], 1i64.to_be_bytes(), [0; 8]]
    );
}
