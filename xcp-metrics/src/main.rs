pub mod hub;
pub mod rpc;

use std::{
    fs,
    os::unix::net::UnixStream,
    path::{Path, PathBuf},
};

use argh::FromArgs;

use futures::{select, FutureExt};
use xcp_metrics_common::protocol;

/// xcp-metrics main daemon
#[derive(FromArgs, Debug)]
struct Args {
    /// logging level
    #[argh(option, short = 'l', default = "tracing::Level::INFO")]
    log_level: tracing::Level,

    /// xcp-metrics socket path
    #[argh(option, short = 'd')]
    daemon_path: Option<PathBuf>,
}

/// Check if the Unix socket is active and unlink it if it isn't.
///
/// Returns true if the socket is active.
fn check_unix_socket(socket_path: &Path) -> anyhow::Result<bool> {
    if !Path::try_exists(&socket_path)? {
        // Socket doesn't exist.
        return Ok(false);
    }

    match UnixStream::connect(&socket_path) {
        Ok(_) => Ok(true),
        Err(e) => {
            if matches!(e.kind(), std::io::ErrorKind::ConnectionRefused) {
                // Unlink socket
                tracing::warn!(
                    socket = socket_path.to_str(),
                    "Unlinking inactive xcp-metrics socket"
                );
                fs::remove_file(socket_path)?;
                Ok(false)
            } else {
                tracing::error!(
                    socket = socket_path.to_str(),
                    "Unable to check xcp-metrics socket status: {e}"
                );
                Err(e.into())
            }
        }
    }
}

fn main() {
    let args: Args = argh::from_env();

    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(args.log_level)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let socket_path = args
        .daemon_path
        .unwrap_or_else(|| protocol::METRICS_SOCKET_PATH.into());

    if check_unix_socket(Path::new(&socket_path)).unwrap() {
        tracing::error!("Unable to start: xcp-metrics socket is active");
        panic!("Unable to start: is xcp-metrics already running ?");
    }

    let hub = hub::MetricsHub::default();
    let (hub_sender, hub_receiver) = flume::unbounded();

    smol::block_on(async {
        select! {
            res = hub.run(hub_receiver).fuse() => tracing::warn!("Hub returned: {res:?}"),
            res = rpc::run(&socket_path, hub_sender).fuse() => tracing::warn!("RPC Socket returned: {res:?}"),
        }
    });

    tracing::info!("Stopping");
}
