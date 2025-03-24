//! xcp-metrics protocol v4 implementation.
//!
//! Unix Domain Socket-based protocol based on CBOR.
//! All CBOR payloads are prefixed with a 4-bytes big-endian length prefix.
//!
//! TODO: Protocol negociation
use std::io::{self, Read, Write};

use compact_str::CompactString;
use futures::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use serde::{Deserialize, Serialize};

use crate::metrics::{Metric, MetricType};

pub const METRICS_SOCKET_PATH: &str = "/var/lib/xcp/xcp-metrics";
pub const MAX_PAYLOAD_SIZE: u32 = 512 * 1024; // 512 Ko

/// Register a new metric family to the hub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFamily {
    pub name: CompactString,
    pub metric_type: MetricType,
    pub unit: CompactString,
    pub help: CompactString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Remove a metric family.
pub struct RemoveFamily {
    pub name: CompactString,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Replace the values of a metric.
pub struct UpdateMetric {
    pub family_name: CompactString,
    pub metric: Metric,
    pub uuid: uuid::Uuid,
}

/// Remove a metric from the hub.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoveMetric {
    pub family_name: CompactString,
    pub uuid: uuid::Uuid,
}

/// Fetch metrics from xcp-metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FetchMetrics {
    /// OpenMetrics v1.0.0
    OpenMetrics1,
    /// OpenMetrics v1.0.0 (Protocol Buffers)
    OpenMetrics1Binary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolMessage {
    CreateFamily(CreateFamily),
    RemoveFamily(RemoveFamily),
    UpdateMetric(UpdateMetric),
    RemoveMetric(RemoveMetric),

    FetchMetrics(FetchMetrics),
}

pub trait XcpMetricsStream {
    fn send_message_raw(&mut self, message: &[u8]) -> io::Result<()>;

    fn send_message(&mut self, message: ProtocolMessage) -> io::Result<()> {
        let mut buffer = vec![];
        ciborium::into_writer(&message, &mut buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        self.send_message_raw(&buffer)
    }

    fn recv_message_raw(&mut self) -> io::Result<Box<[u8]>>;

    fn recv_message(&mut self) -> io::Result<ProtocolMessage> {
        let buffer = self.recv_message_raw()?;

        ciborium::from_reader(buffer.as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
    }
}

impl<S> XcpMetricsStream for S
where
    S: Read + Write,
{
    fn send_message_raw(&mut self, message: &[u8]) -> io::Result<()> {
        self.write_all(&(message.len() as u32).to_be_bytes())?;
        self.write_all(message)?;

        Ok(())
    }

    fn recv_message_raw(&mut self) -> io::Result<Box<[u8]>> {
        let mut prefix = [0u8; 4];
        self.read_exact(&mut prefix)?;

        let len = u32::from_be_bytes(prefix);

        if len > MAX_PAYLOAD_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "Payload is too large !",
            ));
        }

        let mut buffer = vec![0u8; len as usize].into_boxed_slice();
        self.read_exact(&mut buffer)?;

        Ok(buffer)
    }
}

#[allow(async_fn_in_trait)]
pub trait XcpMetricsAsyncStream {
    async fn send_message_raw_async(&mut self, message: &[u8]) -> io::Result<()>;

    async fn send_message_async(&mut self, message: ProtocolMessage) -> io::Result<()> {
        let mut buffer = vec![];
        ciborium::into_writer(&message, &mut buffer)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        self.send_message_raw_async(&buffer).await
    }

    async fn recv_message_raw_async(&mut self) -> io::Result<Box<[u8]>>;

    async fn recv_message_async(&mut self) -> io::Result<ProtocolMessage> {
        let buffer = self.recv_message_raw_async().await?;

        ciborium::from_reader(buffer.as_ref())
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
    }
}

impl<S> XcpMetricsAsyncStream for S
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    async fn send_message_raw_async(&mut self, message: &[u8]) -> io::Result<()> {
        self.write(&(message.len() as u32).to_be_bytes()).await?;
        self.write_all(message).await?;

        Ok(())
    }

    async fn recv_message_raw_async(&mut self) -> io::Result<Box<[u8]>> {
        let mut prefix = [0u8; 4];
        self.read_exact(&mut prefix).await?;

        let len = u32::from_be_bytes(prefix);

        if len > MAX_PAYLOAD_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "Payload is too large !",
            ));
        }

        let mut buffer = vec![0u8; len as usize].into_boxed_slice();
        self.read_exact(&mut buffer).await?;

        Ok(buffer)
    }
}
