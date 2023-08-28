use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    sync::Arc,
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
    /// Port to bind the HTTP server to.
    #[arg(short, long)]
    port: u16,

    /// Path to the xcp-metrics daemon socket to fetch metrics from.
    #[arg(short, long, default_value_t = String::from("/var/lib/xcp/xcp-metrics"))]
    daemon_path: String,
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
    let args = Arc::new(Args::parse());
    let daemon_path = Path::new(&args.daemon_path).to_path_buf();

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

    let server = Server::bind(&SocketAddr::new(
        IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        args.port,
    ))
    .serve(service_fn);

    server.await.expect("Proxy server failure");
}
