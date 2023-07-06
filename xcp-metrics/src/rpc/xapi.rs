use std::sync::Arc;

use tokio::{
    net::UnixStream,
    sync::mpsc,
    task::{self, JoinHandle},
};
use xcp_metrics_common::xapi::{
    self,
    hyper::{
        self,
        service::{make_service_fn, service_fn},
        Body,
    },
    hyperlocal::UnixServerExt,
};

use crate::{hub::HubPushMessage, rpc};

pub async fn start_daemon(
    daemon_name: &str,
    hub_channel: mpsc::UnboundedSender<HubPushMessage>,
) -> anyhow::Result<JoinHandle<()>> {
    let socket_path = xapi::get_module_path(daemon_name);
    let shared: Arc<rpc::RpcShared> = Arc::default();

    let make_service = make_service_fn(move |socket: &UnixStream| {
        let shared = shared.clone();
        println!("{socket:?}");
        let hub_channel = hub_channel.clone();

        async move {
            anyhow::Ok(service_fn(move |request: hyper::Request<Body>| {
                rpc::entrypoint(shared.clone(), hub_channel.clone(), request)
            }))
        }
    });

    let server_task = task::spawn(async move {
        hyper::Server::bind_unix(socket_path)
            .expect("Unable to bind to socket")
            .serve(make_service)
            .await
            .unwrap();
    });

    Ok(server_task)
}
