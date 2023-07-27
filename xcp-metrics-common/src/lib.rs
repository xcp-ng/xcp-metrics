//! xcp-metrics common library
pub mod metrics;
pub mod openmetrics;
pub mod protocol_v3;
pub mod rpc;
pub mod rrdd;
pub mod xapi;

#[cfg(test)]
mod test;