pub mod mock;
pub mod read;
pub mod write;
pub mod watch;
pub mod watch_cache;

#[cfg(not(feature = "xenstore-wip"))]
pub use xenstore_rs as xs;

#[cfg(feature = "xenstore-wip")]
pub use xenstore_rs_wip as xs;