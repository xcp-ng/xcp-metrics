use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use tokio::{
    net::{UnixListener, UnixStream},
    sync::mpsc,
    task::{self, JoinHandle},
};
use xcp_metrics_common::xapi;

use crate::{hub::HubPushMessage, rpc};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
struct ForwardedRequest {}

fn forwarded_handler(
    stream: UnixStream,
    _hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    _shared: Arc<rpc::RpcShared>,
) {
    // Try to read stream
    let Ok(stream) = stream.into_std() else { tracing::error!("Failed to convert tokio stream into std stream."); return };

    let deserializer = Deserializer::from_reader(stream);

    for value in deserializer.into_iter::<serde_json::Value>() {
        tracing::info!("Captured value: {value:?}");
    }
}

#[tracing::instrument(skip(hub_channel))]
pub async fn start_forwarded_socket(
    daemon_name: &str,
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
    shared: Arc<rpc::RpcShared>,
) -> anyhow::Result<JoinHandle<()>> {
    let socket_path = xapi::get_module_path(daemon_name);
    let listener = UnixListener::bind(socket_path)?;

    tracing::info!("Starting forwarded");

    Ok(task::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            tracing::info!("Forwarded request from {addr:?}");

            let hub_channel = hub_channel.clone();
            let shared = shared.clone();
            task::spawn_blocking(|| forwarded_handler(stream, hub_channel, shared));
        }
    }))
}
