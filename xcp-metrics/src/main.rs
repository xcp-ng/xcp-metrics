use providers::{protocol_v2::ProtocolV2Provider, Provider};

mod rpc;
//mod rrdd;
//mod xapi;
pub mod hub;
pub mod providers;

#[tokio::main]
async fn main() {
    let (handle, channel) = hub::MetricsHub::default().start().await;

    ProtocolV2Provider::new("xcp-metrics-plugin-xen").start_provider(channel);

    handle.await.unwrap();
}
