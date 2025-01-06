pub mod hub;
pub mod rpc;

use std::{
    fs,
    path::{Path, PathBuf},
};

use clap::{command, Parser};
use tokio::{net::UnixStream, select};

use xcp_metrics_common::protocol;

/// xcp-metrics main daemon
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Logging level
    #[arg(short, long, default_value_t = tracing::Level::INFO)]
    log_level: tracing::Level,

    /// xcp-metrics socket path
    #[arg(long)]
    daemon_path: Option<PathBuf>,
}

/// Check if the Unix socket is active and unlink it if it isn't.
///
/// Returns true if the socket is active.
async fn check_unix_socket(socket_path: &Path) -> anyhow::Result<bool> {
    if !tokio::fs::try_exists(&socket_path).await? {
        // Socket doesn't exist.
        return Ok(false);
    }

    match UnixStream::connect(&socket_path).await {
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(args.log_level)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let socket_path = args
        .daemon_path
        .unwrap_or_else(|| protocol::METRICS_SOCKET_PATH.into());

    if check_unix_socket(Path::new(&socket_path)).await.unwrap() {
        tracing::error!("Unable to start: xcp-metrics socket is active");
        panic!("Unable to start: is xcp-metrics already running ?");
    }

    let (hub, hub_channel) = hub::MetricsHub::default().start().await;
    let rpc_task = rpc::run(&socket_path, hub_channel);

    select! {
        res = hub => tracing::warn!("Hub returned: {res:?}"),
        res = rpc_task => tracing::warn!("RPC Socket returned: {res:?}"),
    };

    tracing::info!("Stopping");
}
