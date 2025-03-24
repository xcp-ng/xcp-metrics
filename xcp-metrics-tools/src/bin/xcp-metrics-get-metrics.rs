use std::{
    io::{stdout, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
};

use argh::FromArgs;
use xcp_metrics_common::protocol::{self, FetchMetrics, XcpMetricsStream};

/// Tool to get metrics from xcp-metrics in OpenMetrics format.
#[derive(FromArgs, Debug)]
struct Args {
    /// path to the xcp-metrics daemon socket to fetch metrics from.
    #[argh(option, short = 'd')]
    daemon_path: Option<PathBuf>,

    /// whether to use protocol buffers binary format.
    #[argh(switch, short = 'b')]
    binary: bool,
}

fn main() {
    let args: Args = argh::from_env();
    let daemon_path = args
        .daemon_path
        .unwrap_or(protocol::METRICS_SOCKET_PATH.into());

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
