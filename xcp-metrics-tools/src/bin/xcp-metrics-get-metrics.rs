use std::path::PathBuf;

use clap::Parser;
use tokio::io::{stdout, AsyncWriteExt};
use xapi::{
    hyper::{self, body, Body},
    hyperlocal,
    rpc::{
        message::RpcKind, methods::OpenMetricsMethod, write_method_jsonrpc, write_method_xmlrpc,
    },
};

/// Tool to get metrics from xcp-metrics in OpenMetrics format.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the xcp-metrics daemon socket to fetch metrics from.
    #[arg(short, long)]
    daemon_path: Option<PathBuf>,

    /// RPC format to use
    #[arg(long, default_value_t = RpcKind::JsonRpc)]
    rpc_format: RpcKind,

    /// Whether to use protocol buffers binary format.
    #[arg(short, long, default_value_t = false)]
    binary: bool,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let daemon_path = args
        .daemon_path
        .unwrap_or_else(|| xapi::get_module_path("xcp-metrics"));

    let module_uri = hyperlocal::Uri::new(daemon_path, "/");

    let mut rpc_buffer = vec![];
    let method = OpenMetricsMethod {
        protobuf: args.binary,
    };

    match args.rpc_format {
        RpcKind::JsonRpc => write_method_jsonrpc(&mut rpc_buffer, &method).unwrap(),
        RpcKind::XmlRpc => write_method_xmlrpc(&mut rpc_buffer, &method).unwrap(),
    };

    let content_type = match args.rpc_format {
        RpcKind::JsonRpc => "application/json-rpc",
        RpcKind::XmlRpc => "application/xml",
    };

    eprintln!("Sent: {}", String::from_utf8_lossy(&rpc_buffer));

    let request = hyper::Request::builder()
        .uri(hyper::Uri::from(module_uri))
        .method("POST")
        .header("User-agent", "xcp-metrics-get-metrics")
        .header("content-length", rpc_buffer.len())
        .header("content-type", content_type)
        .header("host", "localhost")
        .body(Body::from(rpc_buffer))
        .unwrap();

    let response = hyper::Client::builder()
        .build(hyperlocal::UnixConnector)
        .request(request)
        .await;

    eprintln!("{response:#?}");

    let response = response.unwrap();
    let data = body::to_bytes(response.into_body()).await.unwrap();

    stdout().write_all(&data).await.unwrap();
}
