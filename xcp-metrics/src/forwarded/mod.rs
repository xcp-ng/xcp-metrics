mod response;

use std::{
    collections::HashMap,
    os::fd::{FromRawFd, RawFd},
    slice,
    sync::Arc,
};

use sendfd::RecvWithFd;
use serde::Deserialize;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, UnixListener, UnixStream},
    task::{self, JoinHandle},
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

async fn forwarded_handler(
    stream: UnixStream,
    _shared: Arc<XcpMetricsShared>,
) -> anyhow::Result<()> {
    let (buffer, fd) = task::spawn_blocking(move || {
        let mut buffer = vec![0u8; 10240];
        let mut fd: RawFd = Default::default();

        let std_stream = stream
            .into_std()
            .expect("Unable to convert tokio stream to std stream.");
        let (readed, fds_count) = std_stream
            .recv_with_fd(&mut buffer, slice::from_mut(&mut fd))
            .expect("recv_with_fd failure");

        assert_eq!(fds_count, 1, "fds_count is not 1");

        buffer.shrink_to(readed);

        (buffer.into_boxed_slice(), fd)
    })
    .await?;

    // Get the fd from the forwarded response.
    let mut destination = TcpStream::from_std(unsafe { std::net::TcpStream::from_raw_fd(fd) })?;

    let request: ForwardedRequest = serde_json::from_slice(&buffer)?;
    tracing::info!("Captured request: {request:?}");

    let response = Response::builder()
        .status(StatusCode::OK)
        .body("Hello there !")
        .unwrap();

    let mut string: Vec<u8> = Vec::new();
    response::write_response(&mut string, response)
        .await
        .unwrap();

    tracing::trace!(
        "Sending request to socket:\n{}",
        String::from_utf8_lossy(&string)
    );

    destination.write_all(&string).await.unwrap();

    Ok(())
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

            if let Err(e) = forwarded_handler(stream, shared).await {
                tracing::error!("Forwarded handler failure {e}");
            }
        }
    }))
}
