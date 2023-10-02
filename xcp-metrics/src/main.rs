pub mod forwarded;
pub mod hub;
mod mappings;
pub mod providers;
pub mod publishers;
pub mod rpc;

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use clap::{command, Parser};
use dashmap::DashMap;
use rpc::routes::RpcRoutes;
use tokio::{net::UnixStream, select, sync::mpsc, task::JoinHandle};

use publishers::rrdd::server::{RrddServer, RrddServerMessage};

use xcp_metrics_common::utils::mapping::CustomMapping;

/// Shared xcp-metrics structure.
#[derive(Debug)]
pub struct XcpMetricsShared {
    /// Handles of the tasks associated to each [providers]
    pub plugins: DashMap<Box<str>, JoinHandle<()>>,

    /// Channel to communicate with hub.
    pub hub_channel: mpsc::UnboundedSender<hub::HubPushMessage>,

    /// Channel to communicate with the rrdd compatibility server.
    pub rrdd_channel: mpsc::UnboundedSender<RrddServerMessage>,

    /// List of RPC routes
    pub rpc_routes: RpcRoutes,
}

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

    /// Custom RrddServer v3-to-v2 mapping file.
    #[arg(short, long)]
    mapping_file: Option<PathBuf>,
}

/// Check if the XAPI socket is active and unlink it if it isn't.
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
                    "Unlinking inactive XAPI socket"
                );
                fs::remove_file(socket_path)?;
                Ok(false)
            } else {
                tracing::error!(
                    socket = socket_path.to_str(),
                    "Unable to check XAPI socket status: {e}"
                );
                Err(e.into())
            }
        }
    }
}

/// Load the mappings from file or use default ones (for now).
async fn get_mappings(
    mapping_path: Option<&Path>,
) -> anyhow::Result<HashMap<Box<str>, CustomMapping>> {
    if let Some(path) = mapping_path {
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    } else {
        Ok(mappings::default_mappings())
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

    // Use xcp-rrdd socket path if arg0 is xcp-rrdd and not specified in Args.
    let socket_path = args.daemon_path.unwrap_or_else(|| {
        let Some(arg0) = std::env::args().next() else {
            return xapi::get_module_path("xcp-metrics");
        };

        let arg0_pathname = Path::new(&arg0)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        if arg0_pathname == "xcp-rrdd" {
            tracing::info!("Program name is xcp-rrdd, use xcp-rrdd socket path by default");
            return xapi::get_module_path("xcp-rrdd");
        }

        xapi::get_module_path("xcp-metrics")
    });

    let forwarded_path = format!("{}.forwarded", socket_path.to_string_lossy());

    if check_unix_socket(Path::new(&socket_path)).await.unwrap() {
        tracing::error!("Unable to start: xcp-metrics socket is active");
        panic!("Unable to start: is xcp-metrics already running ?");
    }

    if check_unix_socket(Path::new(&forwarded_path)).await.unwrap() {
        tracing::error!("Unable to start: xcp-metrics.forwarded socket is active");
        panic!("Unable to start: is xcp-metrics already running ?");
    }

    let (hub, hub_channel) = hub::MetricsHub::default().start().await;
    let (rrdd_server, rrdd_channel) =
        RrddServer::new(get_mappings(args.mapping_file.as_deref()).await.unwrap());

    let shared = Arc::new(XcpMetricsShared {
        hub_channel,
        plugins: Default::default(),
        rrdd_channel,
        rpc_routes: Default::default(),
    });

    let socket = rpc::daemon::start_daemon(&socket_path, shared.clone())
        .await
        .unwrap();

    let socket_forwarded =
        forwarded::start_forwarded_socket(Path::new(&forwarded_path), shared.clone())
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
