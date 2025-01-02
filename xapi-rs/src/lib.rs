//! XAPI utilities
pub mod rpc;

#[cfg(feature = "unix")]
pub mod unix;

#[cfg(feature = "http")]
pub mod http;