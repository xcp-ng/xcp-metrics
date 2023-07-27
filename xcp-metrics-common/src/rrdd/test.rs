use std::{
    io::{Read, Write},
    time::{Duration, SystemTime},
};

use indexmap::indexmap;

use crate::rrdd::protocol_v2::{RrddMessageHeader, RrddMetadata, RrddMetadataRaw};

/// Check if metadata stays the same after being encoded then decoded.
#[test]
fn test_metadata_invariance() {
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

/// Check if protocol v2 header stays the same after being encoded then decoded.
/// NOTE: Timestamp precision is not kept.
#[test]
fn test_protocol_v2_invariance() {
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

    let mut reader = buffer.as_slice();
    let header_readed = RrddMessageHeader::parse_from(&mut reader).unwrap();
    assert_eq!(header, header_readed);

    let mut metadata_buffer = vec![0u8; header_readed.metadata_length as usize];
    reader.read_exact(&mut metadata_buffer).unwrap();
    let metadata_raw_readed: RrddMetadataRaw = serde_json::from_slice(&metadata_buffer).unwrap();
    let metadata_readed = metadata_raw_readed.try_into().unwrap();

    assert_eq!(metadata, metadata_readed);
}
