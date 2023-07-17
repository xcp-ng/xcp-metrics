//! XAPI daemon utilities.
mod forwarded;
use std::sync::Arc;

use tokio::{
    net::UnixStream,
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

use crate::{rpc, XcpMetricsShared};

pub async fn start_daemon(
    daemon_name: &str,
    shared: Arc<XcpMetricsShared>,
) -> anyhow::Result<JoinHandle<()>> {
    let socket_path = xapi::get_module_path(daemon_name);

    let make_service = make_service_fn(move |socket: &UnixStream| {
        let shared = shared.clone();
        tracing::debug!("Accepted unix stream {socket:?}");

        async move {
            anyhow::Ok(service_fn(move |request: hyper::Request<Body>| {
                rpc::entrypoint(shared.clone(), request)
            }))
        }
    });

    tracing::info!("Starting");

    let server_task = task::spawn(async move {
        hyper::Server::bind_unix(socket_path)
            .expect("Unable to bind to socket")
            .serve(make_service)
            .await
            .unwrap();
    });

    Ok(server_task)
}

pub use forwarded::start_forwarded_socket;
