mod plugin;

use clap::{command, Parser};
use std::path::PathBuf;
use tokio::net::UnixStream;
use xcp_metrics_common::protocol::METRICS_SOCKET_PATH;
use xenstore_rs::tokio::XsTokio;

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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(args.log_level)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let rpc_stream = match UnixStream::connect(METRICS_SOCKET_PATH).await {
        Ok(stream) => stream,
        Err(e) => {
            tracing::error!("Unable to connect to xcp-metrics: {e}");
            return;
        }
    };

    let xs = match XsTokio::new().await {
        Ok(xs) => xs,
        Err(e) => {
            tracing::error!("Unable to initialize XenStore: {e}");
            return;
        }
    };

    if let Err(e) = plugin::run_plugin(rpc_stream, xs).await {
        tracing::error!("Plugin failure {e}");
    }
}
