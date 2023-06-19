//! xcp-rrdd protocol v2 implementation.
use std::{
    io::{self, ErrorKind, IoSliceMut, Read, Write},
    time::{self, Duration, SystemTime},
};

use super::protocol_common::DataSourceParseError;

#[derive(Debug)]
pub enum RrddProtocolError {
    InvalidConstantString,
    IoError(io::Error),
    UnfinishedHeader,
    DataSourceParse(DataSourceParseError),
}

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
    pub datasource_values_raw: Box<[[u8; 8]]>,
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

/// Read::read_vectored wrapper that makes sure that the buffer has been entirely readed.
fn read_vectored_exact<R: Read>(
    input: &mut R,
    bufs: &mut [IoSliceMut<'_>],
) -> Result<usize, RrddProtocolError> {
    let expected: usize = bufs.iter().map(|buf| buf.len()).sum();
    let mut readed = 0usize;

    while readed != expected {
        let count = match input.read_vectored(bufs) {
            Ok(0) => return Err(RrddProtocolError::UnfinishedHeader),
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => 0,
            Err(e) => return Err(RrddProtocolError::IoError(e)),
        };

        readed += count;
    }

    Ok(readed)
}

impl RrddMessageHeader {
    /// Parse a message header from a readable source.
    //
    // TODO: Maybe consider using only read_exact instead of read_vectored ?
    pub fn parse_from<R: Read>(input: &mut R) -> Result<Self, RrddProtocolError> {
        // Parse the first part
        let mut header_buffer = [0u8; HEADER.len()];
        let mut data_checksum_buffer = [0u8; 4];
        let mut metadata_checksum_buffer = [0u8; 4];
        let mut datasource_count_buffer = [0u8; 4];
        let mut timestamp_buffer = [0u8; 8];

        let mut first_part = [
            IoSliceMut::new(&mut header_buffer),
            IoSliceMut::new(&mut data_checksum_buffer),
            IoSliceMut::new(&mut metadata_checksum_buffer),
            IoSliceMut::new(&mut datasource_count_buffer),
            IoSliceMut::new(&mut timestamp_buffer),
        ];

        let mut header_size = 0;

        header_size += read_vectored_exact(input, &mut first_part)?;

        if HEADER.as_bytes() != header_buffer {
            return Err(RrddProtocolError::InvalidConstantString);
        }

        let data_checksum = u32::from_be_bytes(data_checksum_buffer);
        let metadata_checksum = u32::from_be_bytes(metadata_checksum_buffer);
        let datasource_count = u32::from_be_bytes(datasource_count_buffer);
        let timestamp_epoch = u64::from_be_bytes(timestamp_buffer);

        let timestamp = time::UNIX_EPOCH + Duration::from_secs(timestamp_epoch);

        let mut datasource_values_buffers = vec![[0u8; 8]; datasource_count as usize];

        let mut datasource_values_slice: Vec<IoSliceMut<'_>> = datasource_values_buffers
            .iter_mut()
            .map(|buffer| IoSliceMut::new(buffer))
            .collect();

        header_size += read_vectored_exact(input, &mut datasource_values_slice)?;

        let mut metadata_length_buffer = [0u8; 4];

        input.read_exact(&mut metadata_length_buffer)?;
        header_size += metadata_length_buffer.len();

        let metadata_length = u32::from_be_bytes(metadata_length_buffer);

        Ok(Self {
            data_checksum,
            metadata_checksum,
            timestamp,
            datasource_values_raw: datasource_values_buffers.into_boxed_slice(),
            metadata_length,
            header_size,
        })
    }

    pub fn emit_to<W: Write>(&self, output: &mut W) -> Result<(), RrddProtocolError> {
        let mut buffer = Vec::with_capacity(
            RRDD_HEADER_LENGTH_PART1
            + 4 /* metadata length */
            + 8 * self.datasource_values_raw.len(), /* datasource values */
        );

        buffer.write_all(HEADER.as_bytes())?;
        buffer.write_all(&self.data_checksum.to_be_bytes())?;
        buffer.write_all(&self.metadata_checksum.to_be_bytes())?;
        buffer.write_all(&(self.datasource_values_raw.len() as u32).to_be_bytes())?;

        buffer.write_all(
            &self
                .timestamp
                .duration_since(time::UNIX_EPOCH)
                .expect("Timestamp not representable (before epoch) ?")
                .as_secs()
                .to_be_bytes(),
        )?;

        for value in self.datasource_values_raw.iter() {
            buffer.write_all(value)?;
        }

        buffer.write_all(&self.metadata_length.to_be_bytes())?;

        output.write_all(&buffer)?;

        Ok(())
    }
}
