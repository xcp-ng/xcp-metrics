mod response;

use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use serde_json::Deserializer;
use tokio::{
    net::{UnixListener, UnixStream},
    task::{self, JoinHandle}, runtime::Runtime,
};
use xcp_metrics_common::xapi::{
    self,
    hyper::{Response, StatusCode},
};

use crate::XcpMetricsShared;

/// xapi-project/xen-api/blob/master/ocaml/libs/http-lib/http.ml for reference
#[derive(Clone, Debug, Deserialize)]
struct ForwardedRequest {
    pub m: Box<str>,
    pub uri: Box<str>,
    pub query: HashMap<Box<str>, Box<str>>,
    pub version: Box<str>,
    pub frame: bool,
    pub transfer_encoding: Option<Box<str>>,
    pub accept: Option<Box<str>>,
    pub content_length: Option<usize>,
    pub auth: Option<Box<[Box<str>]>>,
    pub cookie: HashMap<Box<str>, Box<str>>,
    pub task: Option<Box<str>>,
    pub subtask_of: Option<Box<str>>,
    pub content_type: Option<Box<str>>,
    pub host: Option<Box<str>>,
    pub user_agent: Option<Box<str>>,
    pub close: bool,
    pub additional_headers: HashMap<Box<str>, Box<str>>,
    pub body: Option<Box<str>>,
    pub traceparent: Option<Box<str>>,
}

fn forwarded_handler(stream: UnixStream, _shared: Arc<XcpMetricsShared>) {
    // Try to read stream
    let Ok(mut stream) = stream.into_std() else { tracing::error!("Failed to convert tokio stream into std stream."); return };

    let deserializer = Deserializer::from_reader(stream.try_clone().unwrap());

    for value in deserializer.into_iter::<ForwardedRequest>() {
        match value {
            Ok(value) => {
                tracing::info!("Captured request: {value:?}");

                let response = Response::builder()
                    .status(StatusCode::OK)
                    .body("Hello there !")
                    .unwrap();

                Runtime::new().unwrap().block_on(async {
                    response::write_response(&mut stream, &response)
                        .await
                        .unwrap()
                });
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
