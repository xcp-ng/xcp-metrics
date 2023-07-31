pub mod forwarded;
pub mod hub;
pub mod providers;
pub mod publishers;
pub mod rpc;

use dashmap::DashMap;
use std::{fs, sync::Arc};
use tokio::{net::UnixStream, select, sync::mpsc, task::JoinHandle};
use xcp_metrics_common::xapi::XAPI_SOCKET_PATH;

#[derive(Debug)]
pub struct XcpMetricsShared {
    pub plugins: DashMap<Box<str>, JoinHandle<()>>,
    pub hub_channel: mpsc::UnboundedSender<hub::HubPushMessage>,
}

/// Check if the XAPI socket is active and unlink it if it isn't.
///
/// Returns true if the socket is active.
async fn check_unix_socket(daemon_name: &str) -> anyhow::Result<bool> {
    let socket_path = format!("{XAPI_SOCKET_PATH}/{daemon_name}");

    if !tokio::fs::try_exists(&socket_path).await? {
        // Socket doesn't exist.
        return Ok(false);
    }

    match UnixStream::connect(&socket_path).await {
        Ok(_) => Ok(true),
        Err(e) => {
            if matches!(e.kind(), std::io::ErrorKind::ConnectionRefused) {
                // Unlink socket
                tracing::warn!(socket = socket_path, "Unlinking inactive XAPI socket");
                fs::remove_file(&socket_path)?;
                Ok(false)
            } else {
                tracing::error!(
                    socket = socket_path,
                    "Unable to check XAPI socket status: {e}"
                );
                Err(e.into())
            }
        }
    }
}

#[tokio::main]
async fn main() {
    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(tracing::Level::DEBUG)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    if check_unix_socket("xcp-metrics").await.unwrap() {
        tracing::error!("Unable to start: xcp-metrics socket is active");
        panic!("Unable to start: is xcp-metrics already running ?");
    }

    if check_unix_socket("xcp-metrics.forwarded").await.unwrap() {
        tracing::error!("Unable to start: xcp-metrics.forwarded socket is active");
        panic!("Unable to start: is xcp-metrics already running ?");
    }

    let (hub, hub_channel) = hub::MetricsHub::default().start().await;

    let shared = Arc::new(XcpMetricsShared {
        hub_channel,
        plugins: Default::default(),
    });

    let socket = rpc::daemon::start_daemon("xcp-metrics", shared.clone())
        .await
        .unwrap();

    let socket_forwarded = forwarded::start_forwarded_socket("xcp-metrics.forwarded", shared)
        .await
        .unwrap();

    select! {
        res = hub => tracing::warn!("Hub returned: {res:?}"),
        res = socket => tracing::warn!("RPC Socket returned: {res:?}"),
        res = socket_forwarded => tracing::warn!("RPC Forwarded Socket returned {res:?}"),
    };

    tracing::info!("Stopping");
}
