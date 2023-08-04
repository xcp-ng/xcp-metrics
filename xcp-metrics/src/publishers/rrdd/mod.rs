mod entry;
pub mod round_robin;
pub mod server;

use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Copy)]
pub enum RrdXportFilter {
    All,
    AllNoHost,
    VM(uuid::Uuid),
    SR(uuid::Uuid),
}

#[derive(Debug, Clone)]
pub struct RrdXportInfo {
    pub start: SystemTime,
    pub interval: u32,
    pub filter: RrdXportFilter,
}

#[derive(Copy, Clone)]
pub(self) enum Granuality {
    FiveSeconds,
    Minute,
    Hour,
    Day,
}

impl Granuality {
    /// Get the duration covered by this level of granuality.
    pub const fn get_covered_duration(self) -> Duration {
        match self {
            // Duration that can cover the five_seconds buffer (10 minutes).
            Self::FiveSeconds => Duration::from_secs(10 * 60),
            // Duration that can cover the minute buffer (2 hours).
            Self::Minute => Duration::from_secs(2 * 3600),
            // Duration that can cover the hour buffer (1 weeks).
            Self::Hour => Duration::from_secs(24 * 7 * 3600),
            // Duration that can cover the day buffer (1 year).
            Self::Day => Duration::from_secs(24 * 3600 * 365),
        }
    }

    pub const fn get_buffer_size(self) -> usize {
        match self {
            // Size of the per five seconds samples buffer.
            Self::FiveSeconds => 10 * 60 / 5,
            // Size of the per minute samples buffer.
            Self::Minute => 2 * 60,
            // Size of the per hour samples buffer.
            Self::Hour => 7 * 24,
            // Size of the per day samples buffer.
            Self::Day => 365,
        }
    }

    /// Number of five-seconds updates between metrics.
    pub const fn get_five_seconds_interval(self) -> u32 {
        match self {
            Self::FiveSeconds => 1,
            Self::Minute => 60 / 5,
            Self::Hour => 3600 / 5,
            Self::Day => 3600 * 24 / 5,
        }
    }

    /// Interval between metrics updates.
    pub const fn get_interval(self) -> Duration {
        Duration::from_secs(5 * self.get_five_seconds_interval() as u64)
    }
}
