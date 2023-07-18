use std::{collections::HashMap, sync::Arc};

use serde::{de::IgnoredAny, Deserialize, Serialize};
use serde_json::Deserializer;
use tokio::{
    net::{UnixListener, UnixStream},
    task::{self, JoinHandle},
};
use xcp_metrics_common::xapi;

use crate::XcpMetricsShared;

/// xapi-project/xen-api/blob/master/ocaml/libs/http-lib/http.ml for reference
#[derive(Clone, Debug, Deserialize)]
struct ForwardedRequest {
    pub m: Box<str>,
    pub uri: Box<str>,
    pub query: HashMap<Box<str>, Box<str>>,
    pub version: IgnoredAny,           // Box<str>,
    pub frame: IgnoredAny,             // bool
    pub transfer_encoding: IgnoredAny, // Option<Box<str>>,
    pub accept: IgnoredAny,            // Option<Box<str>>,
    pub content_length: IgnoredAny,    // Option<usize>,
    pub auth: IgnoredAny,              // Option<Box<[Box<str>]>>
    pub cookie: IgnoredAny,            // HashMap<Box<str>, Box<str>>
    pub task: Option<Box<str>>,
    pub subtask_of: Option<Box<str>>,
    pub content_type: Option<Box<str>>,
    pub host: Option<Box<str>>,
    pub user_agent: Option<Box<str>>,
    pub close: IgnoredAny, // bool
    pub additional_headers: HashMap<Box<str>, Box<str>>,
    pub body: Option<Box<str>>,
    pub traceparent: IgnoredAny, // Option<Box<str>>,
}

/// xapi-project/xen-api/blob/master/ocaml/libs/http-lib/http.ml for reference
#[derive(Clone, Debug, Serialize)]
struct ForwardedResponse {
    pub version: &'static str,
    pub frame: bool,
    pub code: Box<str>,
    pub message: Box<str>,
    pub content_length: Option<usize>,
    pub task: Option<Box<str>>,
    pub additional_headers: HashMap<Box<str>, Box<str>>,
    pub body: Option<Box<str>>,
}

impl Default for ForwardedResponse {
    fn default() -> Self {
        Self {
            version: "1.1",
            frame: false,
            code: "200".into(),
            message: "OK".into(),
            content_length: None,
            task: None,
            additional_headers: HashMap::default(),
            body: None,
        }
    }
}

fn forwarded_handler(stream: UnixStream, _shared: Arc<XcpMetricsShared>) {
    // Try to read stream
    let Ok(mut stream) = stream.into_std() else { tracing::error!("Failed to convert tokio stream into std stream."); return };

    let deserializer = Deserializer::from_reader(stream.try_clone().unwrap());

    for value in deserializer.into_iter::<ForwardedRequest>() {
        match value {
            Ok(value) => {
                tracing::info!("Captured value: {value:?}");
                let body = "Hello There";

                let test_response = ForwardedResponse {
                    body: Some(body.into()),
                    content_length: Some(body.len()),
                    ..Default::default()
                };

                if let Err(e) = serde_json::to_writer(&mut stream, &test_response) {
                    tracing::error!("Forwarded response error {e}")
                }
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
