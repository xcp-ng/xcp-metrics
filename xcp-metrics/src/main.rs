use std::{
    fs::File,
    io::{Read, Seek, SeekFrom}, collections::HashMap,
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
        file.seek(SeekFrom::Start(header.header_size as u64))
            .unwrap();

        let mut buf = vec![0u8; header.metadata_length as usize];
        file.read_exact(&mut buf).unwrap();

        let json_str = String::from_utf8(buf).unwrap();
        println!("{:}", json_str);

        let metadata_raw: MetadataList = serde_json::from_str(json_str.as_str()).unwrap();
        println!("{metadata_raw:#?}");

        for ds in metadata_raw.datasources {
            let metadata = DataSourceMetadata::try_from(ds.1);
            println!("{metadata:#?}");
        }
    }
}
