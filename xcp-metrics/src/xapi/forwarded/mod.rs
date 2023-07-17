use std::{collections::HashMap, sync::Arc};

use serde::{de::IgnoredAny, Deserialize};
use serde_json::Deserializer;
use tokio::{
    net::{UnixListener, UnixStream},
    task::{self, JoinHandle},
};
use xcp_metrics_common::xapi;

use crate::XcpMetricsShared;

#[derive(Clone, Debug, Deserialize)]
struct ForwardedRequest {
    accept: Box<str>,
    additional_headers: HashMap<Box<str>, Box<str>>,
    auth: Box<[Box<str>]>,
    close: IgnoredAny,  // bool
    cookie: IgnoredAny, // HashMap<Box<str>, Box<str>>
    frame: IgnoredAny,  // bool
    host: Box<str>,
    m: Box<str>,
    query: HashMap<Box<str>, Box<str>>,
    uri: Box<str>,
    user_agent: Box<str>,
    version: IgnoredAny, // Box<str>,
}

fn forwarded_handler(stream: UnixStream, _shared: Arc<XcpMetricsShared>) {
    // Try to read stream
    let Ok(stream) = stream.into_std() else { tracing::error!("Failed to convert tokio stream into std stream."); return };

    let deserializer = Deserializer::from_reader(stream);

    for value in deserializer.into_iter::<ForwardedRequest>() {
        match value {
            Ok(value) => {
                tracing::info!("Captured value: {value:?}")
            }
            Err(e) => tracing::warn!("Forwarded iterator error: {e}"),
        }
    }
}

pub async fn start_forwarded_socket(
    daemon_name: &str,
    shared: Arc<XcpMetricsShared>,
) -> anyhow::Result<JoinHandle<()>> {
    let socket_path = xapi::get_module_path(daemon_name);
    let listener = UnixListener::bind(socket_path)?;

    tracing::info!("Starting forwarded");

    Ok(task::spawn(async move {
        while let Ok((stream, addr)) = listener.accept().await {
            tracing::info!("Forwarded request from {addr:?}");

            let shared = shared.clone();
            task::spawn_blocking(|| forwarded_handler(stream, shared));
        }
    }))
}
