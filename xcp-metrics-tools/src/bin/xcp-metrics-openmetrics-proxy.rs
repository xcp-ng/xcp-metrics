use std::{
    convert::Infallible,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use clap::{command, Parser};

use http::Request;
use hyper::{body::Incoming, server::conn::http1, service::service_fn, Response};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;
use xapi::rpc::{message::RpcKind, methods::rrdd::OpenMetricsMethod};

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

async fn redirect_openmetrics(
    request: Request<Incoming>,
    daemon_path: &Path,
) -> Result<Response<Incoming>, Infallible> {
    // TODO: Consider request supported OpenMetrics versions.
    Ok(xapi::unix::send_rpc_to(
        daemon_path,
        "POST",
        &OpenMetricsMethod { protobuf: false },
        "xcp-metrics-openmetrics-proxy",
        RpcKind::JsonRpc,
    )
    .await
    .expect("RPC failure"))
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let daemon_path = args
        .daemon_path
        .unwrap_or_else(|| xapi::unix::get_module_path("xcp-metrics"));

    let listener = TcpListener::bind(args.addr)
        .await
        .expect("Unable to bind socket");

    // We start a loop to continuously accept incoming connections
    loop {
        let daemon_path = daemon_path.clone();
        let (stream, addr) = listener.accept().await.expect("Unable to accept socket");

        println!("Handling request {addr:?}");

        let io = TokioIo::new(stream);

        tokio::task::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(
                    io,
                    service_fn(|request| redirect_openmetrics(request, &daemon_path)),
                )
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}
