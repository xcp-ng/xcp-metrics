mod rpc;
pub mod xapi;

use async_trait::async_trait;
use tokio::sync::mpsc;
use xcp_metrics_common::xapi::hyper::Body;

use crate::hub::HubPushMessage;

// async_trait: https://github.com/rust-lang/rust/issues/91611 and https://crates.io/crates/async-trait (as proposed by rustc)
#[async_trait]
pub trait XcpRpcRoute {
    async fn run(hub_channel: mpsc::UnboundedSender<HubPushMessage>) -> anyhow::Result<Body>;
}
