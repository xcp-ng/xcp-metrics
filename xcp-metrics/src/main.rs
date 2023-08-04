pub mod forwarded;
pub mod hub;
pub mod providers;
pub mod publishers;
pub mod rpc;

use std::{fs, sync::Arc};

use clap::{command, Parser};
use dashmap::DashMap;
use tokio::{net::UnixStream, select, sync::mpsc, task::JoinHandle};

use publishers::rrdd::server::{RrddServer, RrddServerMessage};
use xcp_metrics_common::xapi::XAPI_SOCKET_PATH;

#[derive(Debug)]
pub struct XcpMetricsShared {
    pub plugins: DashMap<Box<str>, JoinHandle<()>>,
    pub hub_channel: mpsc::UnboundedSender<hub::HubPushMessage>,
    pub rrdd_channel: mpsc::UnboundedSender<RrddServerMessage>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value_t = tracing::Level::INFO)]
    log_level: tracing::Level,

    #[arg(long, default_value_t = String::from("xcp-metrics"))]
    daemon_name: String,
}

/// Check if the XAPI socket is active and unlink it if it isn't.
///
/// Returns true if the socket is active.
async fn check_unix_socket(daemon_name: &str) -> anyhow::Result<bool> {
    let socket_path = format!("{XAPI_SOCKET_PATH}/{daemon_name}");

    if !tokio::fs::try_exists(&socket_path).await? {
        // Socket doesn't exist.
        return Ok(false);
    }

    match UnixStream::connect(&socket_path).await {
        Ok(_) => Ok(true),
        Err(e) => {
            if matches!(e.kind(), std::io::ErrorKind::ConnectionRefused) {
                // Unlink socket
                tracing::warn!(socket = socket_path, "Unlinking inactive XAPI socket");
                fs::remove_file(&socket_path)?;
                Ok(false)
            } else {
                tracing::error!(
                    socket = socket_path,
                    "Unable to check XAPI socket status: {e}"
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

    let forwarded_path = format!("{}.forwarded", args.daemon_name);

    if check_unix_socket(&args.daemon_name).await.unwrap() {
        tracing::error!("Unable to start: xcp-metrics socket is active");
        panic!("Unable to start: is xcp-metrics already running ?");
    }

    if check_unix_socket(&forwarded_path).await.unwrap() {
        tracing::error!("Unable to start: xcp-metrics.forwarded socket is active");
        panic!("Unable to start: is xcp-metrics already running ?");
    }

    let (hub, hub_channel) = hub::MetricsHub::default().start().await;
    let (rrdd_server, rrdd_channel) = RrddServer::new();

    let shared = Arc::new(XcpMetricsShared {
        hub_channel,
        plugins: Default::default(),
        rrdd_channel,
    });

    let socket = rpc::daemon::start_daemon(&args.daemon_name, shared.clone())
        .await
        .unwrap();

    let socket_forwarded = forwarded::start_forwarded_socket(&forwarded_path, shared.clone())
        .await
        .unwrap();

    let rrdd = rrdd_server.start(shared.hub_channel.clone());

    select! {
        res = hub => tracing::warn!("Hub returned: {res:?}"),
        res = socket => tracing::warn!("RPC Socket returned: {res:?}"),
        res = socket_forwarded => tracing::warn!("RPC Forwarded Socket returned {res:?}"),
        res = rrdd => tracing::warn!("RRDD server returned {res:?}")
    };

    tracing::info!("Stopping");
}
