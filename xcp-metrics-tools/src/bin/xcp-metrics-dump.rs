use anyhow::Result;
use std::{fs, io::Read};
use xcp_metrics_common::{
    self,
    rrdd::protocol_v2::{RrddMessageHeader, RrddMetadata, RrddMetadataRaw},
};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    if let Some(path) = args.get(1) {
        println!("Trying to read Rrdd message header...");
        let mut file = fs::File::open(path)?;

        match RrddMessageHeader::parse_from(&mut file) {
            Ok(header) => {
                println!("{:#?}", &header);

                let mut buffer = vec![0u8; header.metadata_length as usize];
                file.read_exact(&mut buffer)?;

                let metadata_raw: RrddMetadataRaw = serde_json::from_slice(&buffer)?;
                let metadata = RrddMetadata::try_from(metadata_raw);

                println!("{metadata:#?}");
            }
            Err(e) => eprintln!("{e:?}"),
        }
    } else {
      println!("Usage: xcp-metrics-dump /dev/shm/metrics/<file>");
    }

    Ok(())
}
