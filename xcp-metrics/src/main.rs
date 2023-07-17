pub mod hub;
pub mod providers;
pub mod publishers;
pub mod rpc;

use rpc::xapi;
use std::sync::Arc;
use tokio::select;

#[tokio::main]
async fn main() {
    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(tracing::Level::DEBUG)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let (hub, channel) = hub::MetricsHub::default().start().await;

    let shared: Arc<rpc::RpcShared> = Arc::default();

    let socket = xapi::start_daemon("xcp-rrdd", channel.clone(), shared.clone())
        .await
        .unwrap();

    let socket_forwarded = xapi::start_forwarded_socket("xcp-rrdd.forwarded", channel, shared)
        .await
        .unwrap();

    select! {
        res = hub => tracing::warn!("Hub returned: {res:?}"),
        res = socket => tracing::warn!("RPC Socket returned: {res:?}"),
        res = socket_forwarded => tracing::warn!("RPC Forwarded Socket returned {res:?}"),
    };

    tracing::info!("Stopping");
}
