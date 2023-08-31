//! OpenMetrics conversion and text export.
pub mod convert;
pub mod text;

pub use convert::openmetrics::*;
pub use prost;

#[cfg(test)]
mod test;
