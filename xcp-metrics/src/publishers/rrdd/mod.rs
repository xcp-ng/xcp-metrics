pub mod round_robin;
pub mod server;

use serde::{Deserialize, Serialize};

use self::round_robin::RoundRobinBuffer;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RrdEntry {
    /// Full entry name (KIND:owner:uuid:metric_name)
    pub name: Box<str>,

    /// Metrics per five seconds (for the past ten minutes)
    pub five_seconds: RoundRobinBuffer<f64>,

    /// Metrics per minute (for the past two hours)
    pub minute: RoundRobinBuffer<f64>,

    /// Metrics per hour (for the past week).
    pub hour: RoundRobinBuffer<f64>,

    /// Metrics per day (for the past year).
    pub day: RoundRobinBuffer<f64>,
}

/// Number of five-seconds updates between minute metrics.
const MINUTE_UPDATES_INTERVAL: u32 = 60 / 5;

/// Number of five-seconds updates between hour metrics.
const HOUR_UPDATES_INTERVAL: u32 = 3600 / 5;

/// Number of five-seconds updates between day metrics.
const DAY_UPDATES_INTERVAL: u32 = 3600 * 24 / 5;

impl RrdEntry {
    /// Size of the per five seconds samples buffer.
    pub(super) const FIVE_SECONDS_BUFFER_SIZE: usize = 10 * 60 / 5;

    /// Size of the per minute samples buffer.
    pub(super) const MINUTE_BUFFER_SIZE: usize = 2 * 60;

    /// Size of the per hour samples buffer.
    pub(super) const HOUR_BUFFER_SIZE: usize = 7 * 24;

    /// Size of the per day samples buffer.
    pub(super) const DAY_BUFFER_SIZE: usize = 365;

    pub fn new(name: Box<str>) -> Self {
        Self {
            name,
            // Per five seconds, past ten minutes.
            five_seconds: RoundRobinBuffer::new(Self::FIVE_SECONDS_BUFFER_SIZE),

            // Per minute, past ten minutes.
            minute: RoundRobinBuffer::new(Self::MINUTE_BUFFER_SIZE),

            // Per hour, past week.
            hour: RoundRobinBuffer::new(Self::HOUR_BUFFER_SIZE),

            // Per day, past year.
            day: RoundRobinBuffer::new(Self::DAY_BUFFER_SIZE),
        }
    }
}
