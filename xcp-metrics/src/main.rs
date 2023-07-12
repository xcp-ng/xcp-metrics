use rpc::xapi;
use tokio::select;

pub mod hub;
pub mod providers;
pub mod publishers;
pub mod rpc;

#[tokio::main]
async fn main() {
    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(tracing::Level::DEBUG)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let (hub, channel) = hub::MetricsHub::default().start().await;
    let socket = xapi::start_daemon("xcp-rrdd", channel).await.unwrap();

    select! {
        res = hub => tracing::warn!("Hub returned: {res:?}"),
        res = socket => tracing::warn!("RPC Socket returned: {res:?}")
    };

    tracing::info!("Stopping");
}
