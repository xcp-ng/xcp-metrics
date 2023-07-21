//! xcp-metrics protocol v3 implementation.
//!
//! For reference : https://github.com/xapi-project/xapi-project.github.io/pull/278

use std::{
    io::{self, Read, Write},
    time::{self, Duration, SystemTime},
};

use crc32fast::Hasher;
use prost::Message;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite};

use crate::{metrics::MetricSet, openmetrics};

#[derive(Debug)]
pub enum ProtocolV3Error {
    IoError(io::Error),
    InvalidHeader,
    InvalidChecksum { got: u32, expected: u32 },
    InvalidTimestamp,
    OpenMetricsParseError(prost::DecodeError),
    OpenMetricsEncodeError(prost::EncodeError),
}

impl From<io::Error> for ProtocolV3Error {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

impl From<prost::DecodeError> for ProtocolV3Error {
    fn from(value: prost::DecodeError) -> Self {
        Self::OpenMetricsParseError(value)
    }
}

impl From<prost::EncodeError> for ProtocolV3Error {
    fn from(value: prost::EncodeError) -> Self {
        Self::OpenMetricsEncodeError(value)
    }
}

impl std::fmt::Display for ProtocolV3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolV3Error::IoError(err) => write!(f, "IO Error: {err}"),
            ProtocolV3Error::InvalidHeader => write!(f, "Invalid header"),
            ProtocolV3Error::InvalidChecksum { expected, got } => {
                write!(f, "Invalid checksum (expected {expected}, got {got}")
            }
            ProtocolV3Error::InvalidTimestamp => write!(f, "Invalid timestamp"),
            ProtocolV3Error::OpenMetricsParseError(err) => write!(f, "Payload parse error: {err}"),
            ProtocolV3Error::OpenMetricsEncodeError(err) => {
                write!(f, "Payload encoding error: {err}")
            }
        }
    }
}

impl std::error::Error for ProtocolV3Error {}

#[derive(Clone, Debug)]
pub struct ProtocolV3Header {
    pub data_checksum: u32,
    pub timestamp: SystemTime,
    pub payload_length: usize,
    crc_state: Hasher,
}

pub const PROTOCOL_V3_HEADER: &[u8; 12] = b"OPENMETRICS1";

impl ProtocolV3Header {
    pub fn parse(raw_header: &[u8; 28]) -> Result<Self, ProtocolV3Error> {
        if raw_header[0..12] != *PROTOCOL_V3_HEADER {
            return Err(ProtocolV3Error::InvalidHeader);
        }

        let data_checksum = u32::from_be_bytes(raw_header[12..16].try_into().unwrap());
        let timestamp_epoch = u64::from_be_bytes(raw_header[16..24].try_into().unwrap());
        let payload_length = u32::from_be_bytes(raw_header[24..28].try_into().unwrap());

        // Compute CRC for first part (from timstamp to the end of the header).
        let mut crc_state = Hasher::new();
        crc_state.update(&raw_header[16..]);

        Ok(Self {
            data_checksum,
            timestamp: time::UNIX_EPOCH
                .checked_add(Duration::from_secs(timestamp_epoch))
                .ok_or(ProtocolV3Error::InvalidTimestamp)?,
            payload_length: payload_length as usize,
            crc_state,
        })
    }

    pub fn parse_from<R: Read>(reader: &mut R) -> Result<Self, ProtocolV3Error> {
        let mut raw_header: [u8; 28] = [0u8; 28];

        reader.read_exact(&mut raw_header)?;

        Self::parse(&raw_header)
    }

    pub async fn parse_async<R: Unpin + AsyncRead>(
        reader: &mut R,
    ) -> Result<Self, ProtocolV3Error> {
        let mut raw_header: [u8; 28] = [0u8; 28];

        tokio::io::AsyncReadExt::read_exact(reader, &mut raw_header).await?;

        Self::parse(&raw_header)
    }

    /// Build a protocol v3 message for `payload_raw`.
    pub fn generate(payload_raw: &[u8], timestamp: Option<SystemTime>) -> [u8; 28] {
        // Build the payload in a buffer, and return it.

        let mut buffer = [0u8; 28];
        let mut writer = buffer.as_mut_slice();

        writer.write_all(PROTOCOL_V3_HEADER).unwrap();

        let timestamp = timestamp.unwrap_or_else(SystemTime::now);
        let timestamp_epoch: u64 = timestamp
            .duration_since(time::UNIX_EPOCH)
            .expect("Non representable time encountered")
            .as_secs();

        let payload_length: u32 = payload_raw.len() as u32;

        // data checksum is not yet computed, write 0 for now, we will replace it later.
        writer.write_all(&0u32.to_be_bytes()).unwrap();

        writer.write_all(&timestamp_epoch.to_be_bytes()).unwrap();
        writer.write_all(&payload_length.to_be_bytes()).unwrap();

        // Compute the checksum
        let mut hasher = Hasher::new();
        hasher.update(&buffer[16..]);
        hasher.update(payload_raw);

        let data_checksum: u32 = hasher.finalize();

        // Replace the data checksum in place, writer cannot be used after this
        buffer[12..16].copy_from_slice(&data_checksum.to_be_bytes());

        buffer
    }

    pub fn check_payload(&self, payload: &[u8], final_checksum: &mut u32) -> bool {
        let mut crc_state = self.crc_state.clone();
        crc_state.update(payload);
        *final_checksum = crc_state.finalize();

        *final_checksum == self.data_checksum
    }
}

pub fn parse_v3<R: Read>(reader: &mut R) -> Result<(ProtocolV3Header, MetricSet), ProtocolV3Error> {
    let header = ProtocolV3Header::parse_from(reader)?;

    let mut payload_data = vec![0u8; header.payload_length];
    reader.read_exact(&mut payload_data)?;

    // Check CRC32
    let mut got = 0;

    if !header.check_payload(&payload_data, &mut got) {
        return Err(ProtocolV3Error::InvalidChecksum {
            expected: header.data_checksum,
            got,
        });
    }

    Ok((
        header,
        openmetrics::MetricSet::decode(payload_data.as_slice())?.into(),
    ))
}

pub async fn parse_v3_async<R: Unpin + AsyncRead>(
    reader: &mut R,
) -> Result<(ProtocolV3Header, MetricSet), ProtocolV3Error> {
    let header = ProtocolV3Header::parse_async(reader).await?;

    let mut payload_data = vec![0u8; header.payload_length];
    reader.read_exact(&mut payload_data).await?;

    // Check CRC32
    let mut got = 0;

    if !header.check_payload(&payload_data, &mut got) {
        return Err(ProtocolV3Error::InvalidChecksum {
            expected: header.data_checksum,
            got,
        });
    }

    Ok((
        header,
        openmetrics::MetricSet::decode(payload_data.as_slice())?.into(),
    ))
}

pub fn generate_v3<W: Write>(
    writer: &mut W,
    timestamp: Option<SystemTime>,
    metrics: MetricSet,
) -> Result<(), ProtocolV3Error> {
    let mut payload_raw = Vec::new();
    openmetrics::MetricSet::from(metrics).encode(&mut payload_raw)?;

    let header = ProtocolV3Header::generate(&payload_raw, timestamp);

    writer.write_all(&header)?;
    writer.write_all(&payload_raw)?;

    Ok(())
}

pub async fn generate_v3_async<W: AsyncWrite + Unpin>(
    writer: &mut W,
    timestamp: Option<SystemTime>,
    metrics: MetricSet,
) -> Result<(), ProtocolV3Error> {
    let mut payload_raw = Vec::new();
    openmetrics::MetricSet::from(metrics).encode(&mut payload_raw)?;

    let header = ProtocolV3Header::generate(&payload_raw, timestamp);

    tokio::io::AsyncWriteExt::write_all(writer, &header).await?;
    tokio::io::AsyncWriteExt::write_all(writer, &payload_raw).await?;

    Ok(())
}
