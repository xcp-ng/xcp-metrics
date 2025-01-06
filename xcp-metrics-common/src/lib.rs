//! xcp-metrics common library
pub mod metrics;
pub mod protocol;
pub mod utils;

#[cfg(feature = "openmetrics")]
pub mod openmetrics;
#[cfg(feature = "rrdd_compat")]
pub mod rrdd;

#[cfg(test)]
mod test;
