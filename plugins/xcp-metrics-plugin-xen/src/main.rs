mod plugin;

use std::{os::unix::net::UnixStream, path::PathBuf};

use argh::FromArgs;
use xcp_metrics_common::protocol::METRICS_SOCKET_PATH;
use xen::hypercall::unix::UnixXenHypercall;

/// xcp-metrics XenStore plugin.
#[derive(Clone, FromArgs, Debug)]
struct Args {
    /// logging level
    #[argh(option, short = 'l', default = "tracing::Level::INFO")]
    log_level: tracing::Level,

    /// target daemon.
    #[argh(option, short = 'd')]
    target: Option<PathBuf>,
}

fn main() {
    let args: Args = argh::from_env();

    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(args.log_level)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let mut rpc_stream =
        match UnixStream::connect(args.target.unwrap_or(METRICS_SOCKET_PATH.into())) {
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
