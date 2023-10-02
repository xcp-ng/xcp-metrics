//! [RrdEntry] definition
use serde::{Deserialize, Serialize};
use xcp_metrics_common::rrdd::protocol_common::DataSourceMetadata;

use super::{round_robin::RoundRobinBuffer, Granuality};

/// A xcp-rrdd metric entry.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RrdEntry {
    /// Full entry name (KIND:owner:uuid:metric_name)
    pub name: Box<str>,

    /// Protocol v2 metadata
    pub metadata: DataSourceMetadata,

    /// Metrics per five seconds (for the past ten minutes)
    pub five_seconds: RoundRobinBuffer<f64>,

    /// Metrics per minute (for the past two hours)
    pub minute: RoundRobinBuffer<f64>,

    /// Metrics per hour (for the past week).
    pub hour: RoundRobinBuffer<f64>,

    /// Metrics per day (for the past year).
    pub day: RoundRobinBuffer<f64>,
}

impl RrdEntry {
    pub fn new(name: Box<str>, metadata: DataSourceMetadata) -> Self {
        Self {
            name,
            metadata,

            // Per five seconds, past ten minutes.
            five_seconds: RoundRobinBuffer::new(
                Granuality::FiveSeconds.get_buffer_size(),
                f64::NAN,
            ),

            // Per minute, past ten minutes.
            minute: RoundRobinBuffer::new(Granuality::Minute.get_buffer_size(), f64::NAN),

            // Per hour, past week.
            hour: RoundRobinBuffer::new(Granuality::Hour.get_buffer_size(), f64::NAN),

            // Per day, past year.
            day: RoundRobinBuffer::new(Granuality::Day.get_buffer_size(), f64::NAN),
        }
    }

    pub(super) fn get_buffer(&self, granuality: Granuality) -> &RoundRobinBuffer<f64> {
        match granuality {
            Granuality::FiveSeconds => &self.five_seconds,
            Granuality::Minute => &self.minute,
            Granuality::Hour => &self.hour,
            Granuality::Day => &self.day,
        }
    }
}
