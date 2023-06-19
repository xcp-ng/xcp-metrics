use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use serde::Deserialize;
use xcp_metrics_common::rrdd::{
    protocol_common::{serde_json, DataSourceMetadata, DataSourceMetadataRaw},
    protocol_v2::RrddMessageHeader,
};

#[derive(Debug, Deserialize)]
struct MetadataList {
    datasources: HashMap<String, DataSourceMetadataRaw>,
}

fn main() {
    let mut file = File::open("xcp-rrdd-xenpm").unwrap();

    let header = RrddMessageHeader::parse_from(&mut file);

    println!("{header:#?}");

    if let Ok(header) = header {
        // Test headers
        let mut header_emited = vec![];
        header.write(&mut header_emited).unwrap();

        println!("{header_emited:?}");

        let header_reparsed = RrddMessageHeader::parse_from(&mut header_emited.as_slice());

        println!("{header_reparsed:#?}");

        assert_eq!(header, header_reparsed.unwrap());

        // Test metadata
        file.seek(SeekFrom::Start(header.header_size as u64))
            .unwrap();

        let mut buf = vec![0u8; header.metadata_length as usize];
        file.read_exact(&mut buf).unwrap();

        let json_str = String::from_utf8(buf).unwrap();
        println!("{:}", json_str);

        let metadata_raw: MetadataList = serde_json::from_str(json_str.as_str()).unwrap();
        println!("{metadata_raw:#?}");

        for ds in metadata_raw.datasources {
            let metadata = DataSourceMetadata::try_from(&ds.1);
            println!("{metadata:#?}");

            if let Ok(metadata) = metadata {
                // Invariance test
                let metadata_serialized =
                    serde_json::to_string(&Into::<DataSourceMetadataRaw>::into(&metadata)).unwrap();

                let metadata_analyzed: DataSourceMetadataRaw =
                    serde_json::from_str(&metadata_serialized).unwrap();
                let metadata_reparsed = DataSourceMetadata::try_from(&metadata_analyzed);

                println!("{metadata_reparsed:#?}");
                assert_eq!(metadata, metadata_reparsed.unwrap());
            }
        }
    }
}
