//! RPC routes and daemon..
pub mod routes;

use std::{convert::Infallible, path::Path, sync::Arc};

use http::{Request, Response, StatusCode};
use http_body::Body;
use http_body_util::Full;
use hyper::{body::Bytes, server::conn::http1, service::service_fn};
use hyper_util::rt::TokioIo;
use tokio::{
    net::UnixListener,
    task::{self, JoinHandle},
};

use xapi::rpc::{self};

use crate::XcpMetricsShared;

const MAX_BODY_LEN: u32 = 2 * 1024 * 1024; // 2 MB

fn build_response(status: StatusCode, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .body(Full::new(body.to_string().into()))
        .unwrap()
}

async fn request_handler<B: Body>(
    shared: Arc<XcpMetricsShared>,
    request: Request<B>,
) -> anyhow::Result<Response<Full<Bytes>>>
where
    <B as Body>::Error: std::fmt::Display,
{
    if let Some(header) = request.headers().get("content-length") {
        let header_len: u32 = header.to_str()?.parse()?;

        if header_len > MAX_BODY_LEN {
            tracing::error!("Got too large payload {header_len} > {MAX_BODY_LEN}");

            return Ok(build_response(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Content Too Large",
            ));
        }
    } else {
        tracing::error!("Got response without content-length");

        return Ok(build_response(
            StatusCode::BAD_REQUEST,
            "No content-length provided",
        ));
    }

    let rpc_request = rpc::message::parse_http_request(request).await?;

    Ok(routes::dispatch(shared, rpc_request).await.map(Full::new))
}

pub async fn entrypoint<B: Body>(
    shared: Arc<XcpMetricsShared>,
    request: Request<B>,
) -> Result<Response<Full<Bytes>>, Infallible>
where
    <B as Body>::Error: std::fmt::Display,
{
    Ok(request_handler(shared, request)
        .await
        .inspect_err(|e| tracing::error!("Internal Server Error: {e}"))
        .unwrap_or_else(|_| {
            build_response(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
        }))
}

pub async fn start_daemon(
    daemon_path: &Path,
    shared: Arc<XcpMetricsShared>,
) -> anyhow::Result<JoinHandle<()>> {
    let unix_stream = UnixListener::bind(daemon_path)?;

    Ok(task::spawn(async move {
        loop {
            let shared = shared.clone();
            let (stream, addr) = unix_stream.accept().await.unwrap();
            tracing::info!("Client {addr:?} connected");

            // Wait for socket to be ready.
            //future::poll_fn(|cx| stream.poll_read_ready(cx)).await.unwrap();

            let io = TokioIo::new(stream);

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(|body| entrypoint(shared.clone(), body)))
                    .await
                {
                    tracing::error!("Error servicing connection {addr:?}: {err:?}");
                }
            });
        }
    }))
}
