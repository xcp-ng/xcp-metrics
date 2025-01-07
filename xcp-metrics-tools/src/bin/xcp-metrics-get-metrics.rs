use std::{
    io::{stdout, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use clap::Parser;
use xcp_metrics_common::protocol::{self, FetchMetrics, XcpMetricsStream};

/// Tool to get metrics from xcp-metrics in OpenMetrics format.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the xcp-metrics daemon socket to fetch metrics from.
    #[arg(short, long)]
    daemon_path: Option<PathBuf>,

    /// Whether to use protocol buffers binary format.
    #[arg(short, long, default_value_t = false)]
    binary: bool,
}

fn main() {
    let args = Args::parse();
    let daemon_path = args
        .daemon_path
        .unwrap_or_else(|| protocol::METRICS_SOCKET_PATH.into());

    let mut client = UnixStream::connect(daemon_path).expect("Unable to connect to daemon");

    client
        .send_message(protocol::ProtocolMessage::FetchMetrics(if args.binary {
            FetchMetrics::OpenMetrics1Binary
        } else {
            FetchMetrics::OpenMetrics1
        }))
        .unwrap();

    let data = client
        .recv_message_raw()
        .expect("Unable to receive daemon response");

    stdout().write_all(&data).expect("Can't write output");
}
