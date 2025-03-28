//! xcp-rrdd protocol v2 implementation.
//!
//! For reference: <https://xapi-project.github.io/rrdd/design/plugin-protocol-v2.html>
use std::{
    io::{self, Read, Write},
    time::{self, Duration, SystemTime},
};

use futures::io::AsyncRead;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::protocol_common::{
    DataSourceMetadata, DataSourceMetadataRaw, DataSourceParseError, DataSourceValue,
};

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

/// A parsed Rrdd message header (v2).
#[derive(PartialEq, Eq, Debug)]
pub struct RrddMessageHeader {
    pub data_checksum: u32,
    pub metadata_checksum: u32,
    pub timestamp: SystemTime,
    pub values: Box<[[u8; 8]]>,
    pub metadata_length: u32,
    pub header_size: usize,
}

pub const PROTOCOL_V2_HEADER: &[u8; 11] = b"DATASOURCES";

/// Size of the first part of the header (before data source values and metadata length)
const RRDD_HEADER_LENGTH_PART1: usize =
    PROTOCOL_V2_HEADER.len()
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
        let mut timestamp_buffer = [0u8; 8];

        // Read the first part (all fields until datasource values)
        let mut first_part_buffer = [0u8; RRDD_HEADER_LENGTH_PART1];
        input.read_exact(&mut first_part_buffer)?;

        let (data_checksum, metadata_checksum, values_count, timestamp) =
            Self::parse_first_part(first_part_buffer, &mut timestamp_buffer)?;

        // Second part (values and metadata length)
        let mut second_part_buffer =
            vec![0u8; (8 * values_count + 4/* metadata length */) as usize];
        input.read_exact(&mut second_part_buffer)?;

        let (values, metadata_length) = Self::parse_second_part(
            &second_part_buffer,
            values_count,
            timestamp_buffer,
            data_checksum,
        )?;

        Ok(Self {
            data_checksum,
            header_size: first_part_buffer.len() + second_part_buffer.len(),
            metadata_checksum,
            metadata_length,
            timestamp,
            values,
        })
    }

    /// Parse a message header from a readable source (async variant).
    pub async fn parse_async<R: AsyncRead + Unpin>(
        input: &mut R,
    ) -> Result<Self, RrddProtocolError> {
        let mut timestamp_buffer = [0u8; 8];

        // Read the first part (all fields until datasource values)
        let mut first_part_buffer = [0u8; RRDD_HEADER_LENGTH_PART1];
        futures::AsyncReadExt::read_exact(input, &mut first_part_buffer).await?;

        let (data_checksum, metadata_checksum, values_count, timestamp) =
            Self::parse_first_part(first_part_buffer, &mut timestamp_buffer)?;

        // Second part (values and metadata length)
        let mut second_part_buffer =
            vec![0u8; (8 * values_count + 4/* metadata length */) as usize];
        futures::AsyncReadExt::read_exact(input, &mut second_part_buffer).await?;

        let (values, metadata_length) = Self::parse_second_part(
            &second_part_buffer,
            values_count,
            timestamp_buffer,
            data_checksum,
        )?;

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
        let metadata = serde_json::to_string(&RrddMetadataRaw::from(metadata))
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

        buffer.write_all(PROTOCOL_V2_HEADER)?;
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

    fn parse_first_part(
        first_part_buffer: [u8; 31],
        timestamp_buffer: &mut [u8; 8],
    ) -> Result<(u32, u32, u32, SystemTime), RrddProtocolError> {
        let mut header_buffer = [0u8; PROTOCOL_V2_HEADER.len()];
        let mut data_checksum_buffer = [0u8; 4];
        let mut metadata_checksum_buffer = [0u8; 4];
        let mut values_count_buffer = [0u8; 4];

        // note: slices implements Read.
        let mut first_part_slice = first_part_buffer.as_slice();
        first_part_slice.read_exact(&mut header_buffer)?;

        if *PROTOCOL_V2_HEADER != header_buffer {
            return Err(RrddProtocolError::InvalidConstantString);
        }

        first_part_slice.read_exact(&mut data_checksum_buffer)?;
        let data_checksum = u32::from_be_bytes(data_checksum_buffer);

        first_part_slice.read_exact(&mut metadata_checksum_buffer)?;
        let metadata_checksum = u32::from_be_bytes(metadata_checksum_buffer);

        first_part_slice.read_exact(&mut values_count_buffer)?;
        let values_count = u32::from_be_bytes(values_count_buffer);

        first_part_slice.read_exact(timestamp_buffer)?;
        let timestamp_epoch = u64::from_be_bytes(*timestamp_buffer);

        let timestamp = time::UNIX_EPOCH + Duration::from_secs(timestamp_epoch);

        Ok((data_checksum, metadata_checksum, values_count, timestamp))
    }

    fn parse_second_part(
        second_part_buffer: &[u8],
        values_count: u32,
        timestamp_buffer: [u8; 8],
        data_checksum: u32,
    ) -> Result<(Box<[[u8; 8]]>, u32), RrddProtocolError> {
        // Split values and metadata
        let (values_buffer, metadata_length_buffer) =
            second_part_buffer.split_at(8 * values_count as usize);

        // Check data checksum
        let mut data_checksum_hasher = crc32fast::Hasher::new();
        data_checksum_hasher.update(&timestamp_buffer);
        data_checksum_hasher.update(values_buffer);

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
            u32::from_be_bytes(<[u8; 4]>::try_from(metadata_length_buffer).unwrap());

        Ok((values, metadata_length))
    }
}

/// A non-parsed metadata (datasource list).
#[derive(Serialize, Deserialize)]
pub struct RrddMetadataRaw {
    pub datasources: IndexMap<Box<str>, DataSourceMetadataRaw>,
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

/// A parsed metadata (datasource list)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RrddMetadata {
    pub datasources: IndexMap<Box<str>, DataSourceMetadata>,
}

impl TryFrom<RrddMetadataRaw> for RrddMetadata {
    type Error = DataSourceParseError;

    fn try_from(value: RrddMetadataRaw) -> Result<Self, Self::Error> {
        let mut datasources: IndexMap<Box<str>, DataSourceMetadata> = IndexMap::default();

        for (name, ds) in value.datasources.into_iter() {
            datasources.insert(name, (&ds).try_into()?);
        }

        Ok(Self { datasources })
    }
}

/// Update `header` values with ones provided by `values`.
pub fn values_to_raw(values: &[DataSourceValue]) -> Box<[[u8; 8]]> {
    values
        .iter()
        .map(|value| match *value {
            DataSourceValue::Int64(n) => n.to_be_bytes(),
            DataSourceValue::Float(f) => f.to_be_bytes(),
            DataSourceValue::Undefined => [0; 8],
        })
        .collect()
}
