pub mod mock;
pub mod read;
pub mod watch;
pub mod watch_cache;
pub mod write;

#[cfg(not(feature = "xenstore-wip"))]
pub use xenstore_rs as xs;

#[cfg(feature = "xenstore-wip")]
pub use xenstore_rs_wip as xs;
