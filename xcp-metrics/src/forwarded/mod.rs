//! Forwarded request support.
mod request;
mod response;
mod routes;

use std::{
    os::fd::{FromRawFd, RawFd},
    slice,
    sync::Arc, path::Path,
};

use sendfd::RecvWithFd;
use tokio::{
    io::AsyncWriteExt,
    net::{TcpStream, UnixListener, UnixStream},
    task::{self, JoinHandle},
};

use crate::{
    forwarded::{request::ForwardedRequest, routes::route_forwarded},
    XcpMetricsShared,
};

async fn forwarded_handler(
    stream: UnixStream,
    shared: Arc<XcpMetricsShared>,
) -> anyhow::Result<()> {
    let (buffer, fd) = task::spawn_blocking(move || {
        let mut buffer = vec![0u8; 10240];
        let mut fd: RawFd = RawFd::default();

        let std_stream = stream
            .into_std()
            .expect("Unable to convert tokio stream to std stream.");
        let (readed, fds_count) = std_stream
            .recv_with_fd(&mut buffer, slice::from_mut(&mut fd))
            .expect("recv_with_fd failure");

        assert_eq!(fds_count, 1, "fds_count is not 1");

        buffer.resize(readed, 0);

        (buffer.into_boxed_slice(), fd)
    })
    .await?;

    // Get the fd from the forwarded response.
    let std_destination = unsafe { std::net::TcpStream::from_raw_fd(fd) };
    std_destination.set_nonblocking(true)?;

    let mut destination = TcpStream::from_std(std_destination)?;

    let request: ForwardedRequest = serde_json::from_slice(&buffer)?;
    tracing::info!("Captured request: {request:?}");

    let response = route_forwarded(shared.clone(), request).await;

    let mut string: Vec<u8> = Vec::new();
    response::write_response(&mut string, response?)
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
    daemon_path: &Path,
    shared: Arc<XcpMetricsShared>,
) -> anyhow::Result<JoinHandle<()>> {
    let daemon_path = daemon_path.to_path_buf();
    let listener = UnixListener::bind(daemon_path)?;

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
