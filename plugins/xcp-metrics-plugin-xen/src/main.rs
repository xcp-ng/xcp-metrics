mod plugin;

use std::{os::unix::net::UnixStream, path::PathBuf};

use clap::{command, Parser};
use xcp_metrics_common::protocol::METRICS_SOCKET_PATH;
use xen::hypercall::unix::UnixXenHypercall;

/// xcp-metrics XenStore plugin.
#[derive(Clone, Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Logging level
    #[arg(short, long, default_value_t = tracing::Level::INFO)]
    log_level: tracing::Level,

    /// Target daemon.
    #[arg(short, long)]
    target: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(args.log_level)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let mut rpc_stream = match UnixStream::connect(METRICS_SOCKET_PATH) {
        Ok(stream) => stream,
        Err(e) => {
            tracing::error!("Unable to connect to xcp-metrics: {e}");
            return;
        }
    };

    let hyp = match UnixXenHypercall::new() {
        Ok(xs) => xs,
        Err(e) => {
            tracing::error!("Unable to initialize xen privcmd: {e}");
            return;
        }
    };

    if let Err(e) = plugin::run_plugin(&mut rpc_stream, &hyp) {
        tracing::error!("Plugin failure {e}");
    }
}
