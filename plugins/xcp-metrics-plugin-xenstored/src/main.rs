mod plugin;

use clap::{command, Parser};
use std::path::Path;

use xcp_metrics_plugin_common::{
    plugin::run_hybrid,
    xenstore::xs::{Xs, XsOpenFlags},
};

use plugin::XenStorePlugin;

/// xcp-metrics XenStore plugin.
#[derive(Clone, Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Logging level
    #[arg(short, long, default_value_t = tracing::Level::INFO)]
    log_level: tracing::Level,

    /// Target daemon.
    #[arg(short, long, default_value_t = String::from("xcp-rrdd"))]
    target: String,

    /// Used protocol
    #[arg(short, long, default_value_t = 2)]
    protocol: u32,
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

    let xs = match Xs::new(XsOpenFlags::ReadOnly) {
        Ok(xs) => xs,
        Err(e) => {
            tracing::error!("Unable to initialize XenStore {e}");
            return;
        }
    };

    run_hybrid(
        XenStorePlugin::new(&xs),
        Some(&Path::new(&args.target)),
        args.protocol,
    )
    .await;
}
