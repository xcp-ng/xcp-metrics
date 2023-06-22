//! xcp-rrdd protocol v2 implementation.
use std::{
    io::{self, Read, Write},
    time::{self, Duration, SystemTime},
};

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::protocol_common::{DataSourceMetadata, DataSourceMetadataRaw, DataSourceParseError};

pub use indexmap;

#[derive(Debug)]
pub enum RrddProtocolError {
    InvalidConstantString,
    IoError(io::Error),
    UnfinishedHeader,
    DataSourceParse(DataSourceParseError),
    NonMatchingLength,
    InvalidChecksum,
}

impl std::fmt::Display for RrddProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for RrddProtocolError {}

impl From<io::Error> for RrddProtocolError {
    fn from(value: io::Error) -> Self {
        RrddProtocolError::IoError(value)
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct RrddMessageHeader {
    pub data_checksum: u32,
    pub metadata_checksum: u32,
    pub timestamp: SystemTime,
    pub values: Box<[[u8; 8]]>,
    pub metadata_length: u32,
    pub header_size: usize,
}

const HEADER: &str = "DATASOURCES";

/// Size of the first part of the header (before data source values and metadata length)
const RRDD_HEADER_LENGTH_PART1: usize =
    HEADER.len()
    + 4 /* data checksum */
    + 4 /* metadata checksum */
    + 4 /* number of data source */
    + 8 /* timestamp */
    ;

fn compute_data_checksum(timestamp: SystemTime, values: &[[u8; 8]]) -> u32 {
    let timestamp_buffer = timestamp
        .duration_since(time::UNIX_EPOCH)
        .expect("Timestamp not representable (before epoch) ?")
        .as_secs()
        .to_be_bytes();

    let timestamp_datasource_concat: Vec<u8> = [timestamp_buffer.into(), values.concat()].concat();

    crc32fast::hash(&timestamp_datasource_concat)
}

impl RrddMessageHeader {
    /// Parse a message header from a readable source.
    pub fn parse_from<R: Read>(input: &mut R) -> Result<Self, RrddProtocolError> {
        let mut header_buffer = [0u8; HEADER.len()];
        let mut data_checksum_buffer = [0u8; 4];
        let mut metadata_checksum_buffer = [0u8; 4];
        let mut values_count_buffer = [0u8; 4];
        let mut timestamp_buffer = [0u8; 8];

        // Read the first part (all fields until datasource values)
        let mut first_part_buffer = [0u8; RRDD_HEADER_LENGTH_PART1];
        input.read_exact(&mut first_part_buffer)?;

        // note: slices implements Read.
        let mut first_part_slice = first_part_buffer.as_slice();

        first_part_slice.read_exact(&mut header_buffer)?;

        if HEADER.as_bytes() != header_buffer {
            return Err(RrddProtocolError::InvalidConstantString);
        }

        first_part_slice.read_exact(&mut data_checksum_buffer)?;
        let data_checksum = u32::from_be_bytes(data_checksum_buffer);

        first_part_slice.read_exact(&mut metadata_checksum_buffer)?;
        let metadata_checksum = u32::from_be_bytes(metadata_checksum_buffer);

        first_part_slice.read_exact(&mut values_count_buffer)?;
        let values_count = u32::from_be_bytes(values_count_buffer);

        first_part_slice.read_exact(&mut timestamp_buffer)?;
        let timestamp_epoch = u64::from_be_bytes(timestamp_buffer);

        let timestamp = time::UNIX_EPOCH + Duration::from_secs(timestamp_epoch);

        // Second part (values and metadata length)
        let mut second_part_buffer =
            vec![0u8; (8 * values_count + 4/* metadata length */) as usize];
        input.read_exact(&mut second_part_buffer)?;

        // Split values and metadata
        let (values_buffer, metadata_length_buffer) =
            second_part_buffer.split_at(8 * values_count as usize);

        // Check data checksum
        let mut data_checksum_hasher = crc32fast::Hasher::new();
        data_checksum_hasher.update(&timestamp_buffer);
        data_checksum_hasher.update(&values_buffer);

        if data_checksum != data_checksum_hasher.finalize() {
            return Err(RrddProtocolError::InvalidChecksum);
        }

        // TODO: Consider using slice::array_chunks when stabilized.
        // https://github.com/rust-lang/rust/issues/74985
        let values: Box<[[u8; 8]]> = values_buffer
            .chunks_exact(8)
            .map(|slice| slice.try_into().unwrap())
            .collect();

        let metadata_length =
            u32::from_be_bytes(TryInto::<[u8; 4]>::try_into(metadata_length_buffer).unwrap());

        Ok(Self {
            data_checksum,
            header_size: first_part_buffer.len() + second_part_buffer.len(),
            metadata_checksum,
            metadata_length,
            timestamp,
            values,
        })
    }

    /// Generate a protocol v2 header along with its matching serialized metadata.
    pub fn generate(values: &[[u8; 8]], metadata: RrddMetadata) -> (Self, Box<str>) {
        let timestamp = SystemTime::now();
        let metadata = serde_json::to_string(&Into::<RrddMetadataRaw>::into(metadata))
            .expect("serde_json::to_string failure");

        (
            Self {
                data_checksum: compute_data_checksum(timestamp, values),
                values: values.into(),
                header_size: RRDD_HEADER_LENGTH_PART1 + 4 /* metadata length */ + 8 * values.len(), /* datasource values */
                metadata_checksum: crc32fast::hash(metadata.as_bytes()),
                metadata_length: metadata.len() as u32,
                timestamp,
            },
            metadata.into_boxed_str(),
        )
    }

    /// Update values and timestamp.
    pub fn update_values(&mut self, values: &[[u8; 8]]) -> Result<(), RrddProtocolError> {
        if values.len() != self.values.len() {
            return Err(RrddProtocolError::NonMatchingLength);
        }

        self.timestamp = SystemTime::now();
        self.data_checksum = compute_data_checksum(self.timestamp, values);

        // update old values with newer ones
        self.values.copy_from_slice(values);

        Ok(())
    }

    /// Write the full header to `output`.
    pub fn write<W: Write>(&self, output: &mut W) -> Result<(), RrddProtocolError> {
        let mut buffer = Vec::with_capacity(
            RRDD_HEADER_LENGTH_PART1
            + 4 /* metadata length */
            + 8 * self.values.len(), /* datasource values */
        );

        buffer.write_all(HEADER.as_bytes())?;
        buffer.write_all(&self.data_checksum.to_be_bytes())?;
        buffer.write_all(&self.metadata_checksum.to_be_bytes())?;
        buffer.write_all(&(self.values.len() as u32).to_be_bytes())?;

        buffer.write_all(
            &self
                .timestamp
                .duration_since(time::UNIX_EPOCH)
                .expect("Timestamp not representable (before epoch) ?")
                .as_secs()
                .to_be_bytes(),
        )?;

        for value in self.values.iter() {
            buffer.write_all(value)?;
        }

        buffer.write_all(&self.metadata_length.to_be_bytes())?;

        output.write_all(&buffer)?;

        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct RrddMetadataRaw {
    pub datasources: IndexMap<String, DataSourceMetadataRaw>,
}

impl From<RrddMetadata> for RrddMetadataRaw {
    fn from(value: RrddMetadata) -> Self {
        Self {
            datasources: value
                .datasources
                .into_iter()
                .map(|(name, ds)| (name, (&ds).into()))
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RrddMetadata {
    pub datasources: IndexMap<String, DataSourceMetadata>,
}

impl TryFrom<RrddMetadataRaw> for RrddMetadata {
    type Error = DataSourceParseError;

    fn try_from(value: RrddMetadataRaw) -> Result<Self, Self::Error> {
        let mut datasources: IndexMap<String, DataSourceMetadata> = IndexMap::default();

        for (name, ds) in value.datasources.into_iter() {
            datasources.insert(name, (&ds).try_into()?);
        }

        Ok(Self { datasources })
    }
}
