use std::path::PathBuf;

use clap::Parser;
use http_body_util::BodyExt;
use tokio::io::{stdout, AsyncWriteExt};
use xapi::rpc::{message::RpcKind, methods::rrdd::OpenMetricsMethod};

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
        .unwrap_or_else(|| xapi::unix::get_module_path("xcp-metrics"));

    let method = OpenMetricsMethod {
        protobuf: args.binary,
    };

    let response = xapi::unix::send_rpc_to(
        &daemon_path,
        "POST",
        &method,
        "xcp-metrics-get-metrics",
        RpcKind::JsonRpc,
    )
    .await;

    eprintln!("{response:#?}");

    let data = response
        .unwrap()
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes();

    stdout().write_all(&data).await.unwrap();
}
