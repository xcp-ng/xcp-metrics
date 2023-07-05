use rpc::xapi;
use tokio::select;

pub mod hub;
pub mod providers;
pub mod publishers;
mod rpc;

#[tokio::main]
async fn main() {
    let (hub, channel) = hub::MetricsHub::default().start().await;
    let socket = xapi::start_daemon("xcp-rrdd", channel).await.unwrap();

    select! {
        res = hub => println!("Hub returned: {res:?}"),
        res = socket => println!("RPC Socket returned: {res:?}")
    };

    println!("Stopping");
}
