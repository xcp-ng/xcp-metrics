use std::{
    convert::Infallible,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use clap::{command, Parser};

use http::{header, Request};
use http_body_util::Full;
use hyper::{
    body::{Bytes, Incoming},
    server::conn::http1,
    service::service_fn,
    Response,
};
use hyper_util::rt::TokioIo;
use tokio::{
    net::{TcpListener, UnixStream},
    sync::Mutex,
};
use xcp_metrics_common::protocol::{
    FetchMetrics, ProtocolMessage, XcpMetricsAsyncStream, METRICS_SOCKET_PATH,
};

/// OpenMetrics http proxy, used to provide metrics for collectors such as Prometheus.
#[derive(Clone, Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Adress to bind the HTTP server to.
    #[arg(short, long, default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 8080))]
    addr: SocketAddr,

    /// Path to the xcp-metrics daemon socket to fetch metrics from.
    #[arg(short, long)]
    daemon_path: Option<PathBuf>,
}

const OPENMETRICS_TEXT_CONTENT_TYPE: &str =
    "application/openmetrics-text; version=1.0.0; charset=utf-8";

async fn redirect_openmetrics(
    request: Request<Incoming>,
    rpc_stream: Arc<Mutex<UnixStream>>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    // TODO: Consider request supported OpenMetrics versions.
    let mut rpc_stream = rpc_stream.lock().await;

    rpc_stream
        .send_message_async(ProtocolMessage::FetchMetrics(FetchMetrics::OpenMetrics1))
        .await
        .unwrap();

    let data = rpc_stream.recv_message_raw_async().await.unwrap();
    drop(rpc_stream);

    Ok(Response::builder()
        .header(header::CONTENT_TYPE, OPENMETRICS_TEXT_CONTENT_TYPE)
        .header(header::CONTENT_LENGTH, data.len())
        .header(header::HOST, "xcp-metrics openmetrics proxy")
        .body(Full::new(data.into()))
        .unwrap())
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let daemon_path = args
        .daemon_path
        .unwrap_or_else(|| METRICS_SOCKET_PATH.into());

    let rpc_stream = Arc::new(Mutex::new(
        UnixStream::connect(daemon_path)
            .await
            .expect("Unable to connect to xcp-metrics"),
    ));

    let listener = TcpListener::bind(args.addr)
        .await
        .expect("Unable to bind socket");

    // We start a loop to continuously accept incoming connections
    loop {
        let rpc_stream = rpc_stream.clone();
        let (stream, addr) = listener.accept().await.expect("Unable to accept socket");

        println!("Handling request {addr:?}");

        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(|request| redirect_openmetrics(request, rpc_stream.clone())),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
