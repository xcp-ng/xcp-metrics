use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};

use clap::{command, Parser};
use hyper::{
    server::{conn::AddrStream, Server},
    service::{make_service_fn, service_fn},
    Body, Request, Response,
};

use xapi::rpc::methods::OpenMetricsMethod;

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
    request: Request<Body>,
    daemon_path: &Path,
) -> anyhow::Result<Response<Body>> {
    xapi::send_jsonrpc_to(
        daemon_path,
        "POST",
        &OpenMetricsMethod::default(),
        "xcp-metrics-openmetrics-proxy",
    )
    .await
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let daemon_path = args
        .daemon_path
        .unwrap_or_else(|| xapi::get_module_path("xcp-metrics"));

    let service_fn = make_service_fn(|addr: &AddrStream| {
        println!("Handling request {:?}", addr);
        let daemon_path = daemon_path.clone();

        async {
            anyhow::Ok(service_fn(move |request| {
                let daemon_path = daemon_path.clone();
                async move { redirect_openmetrics(request, &daemon_path).await }
            }))
        }
    });

    let server = Server::bind(&args.addr).serve(service_fn);

    server.await.expect("Proxy server failure");
}
