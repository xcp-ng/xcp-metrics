use anyhow::Result;
use std::{
    fs,
    io::{Read, Seek},
};
use xcp_metrics_common::{
    self,
    rrdd::protocol_v2::{self, RrddMessageHeader, RrddMetadata, RrddMetadataRaw},
};

fn read_protocol_v2(mut file: fs::File) -> Result<(), anyhow::Error> {
    let header = RrddMessageHeader::parse_from(&mut file)?;

    println!("{:#?}", &header);

    let mut buffer = vec![0u8; header.metadata_length as usize];
    file.read_exact(&mut buffer)?;

    let metadata_raw: RrddMetadataRaw = serde_json::from_slice(&buffer)?;
    let metadata = RrddMetadata::try_from(metadata_raw);

    println!("{metadata:#?}");

    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if let Some(path) = args.get(1) {
        println!("Trying to read message header...");
        let mut file = fs::File::open(path)?;

        let mut file_header = [0u8; 12];
        file.read_exact(&mut file_header)?;

        file.rewind()?;

        if file_header[..11] == *protocol_v2::PROTOCOL_V2_HEADER {
            println!("Detected protocol v2");
            read_protocol_v2(file)?;
        } else {
            println!("Unknown file header");
        }
    } else {
        println!("Usage: xcp-metrics-dump /dev/shm/metrics/<file>");
    }

    Ok(())
}
