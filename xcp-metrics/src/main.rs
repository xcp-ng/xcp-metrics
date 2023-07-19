pub mod forwarded;
pub mod hub;
pub mod providers;
pub mod publishers;
pub mod rpc;

use dashmap::DashMap;
use std::sync::Arc;
use tokio::{select, sync::mpsc, task::JoinHandle};

#[derive(Debug)]
pub struct XcpMetricsShared {
    pub plugins: DashMap<Box<str>, JoinHandle<()>>,
    pub hub_channel: mpsc::UnboundedSender<hub::HubPushMessage>,
}

#[tokio::main]
async fn main() {
    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(tracing::Level::DEBUG)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let (hub, hub_channel) = hub::MetricsHub::default().start().await;

    let shared = Arc::new(XcpMetricsShared {
        hub_channel,
        plugins: Default::default(),
    });

    let socket = rpc::daemon::start_daemon("xcp-rrdd", shared.clone())
        .await
        .unwrap();

    let socket_forwarded = forwarded::start_forwarded_socket("xcp-rrdd.forwarded", shared)
        .await
        .unwrap();

    select! {
        res = hub => tracing::warn!("Hub returned: {res:?}"),
        res = socket => tracing::warn!("RPC Socket returned: {res:?}"),
        res = socket_forwarded => tracing::warn!("RPC Forwarded Socket returned {res:?}"),
    };

    tracing::info!("Stopping");
}
