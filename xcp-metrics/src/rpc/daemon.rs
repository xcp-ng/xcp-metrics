//! RPC daemon procedures.
use std::{path::Path, sync::Arc};

use tokio::{
    net::UnixStream,
    task::{self, JoinHandle},
};
use xapi::{
    hyper::{
        self,
        service::{make_service_fn, service_fn},
        Body,
    },
    hyperlocal::UnixServerExt,
};

use crate::{rpc, XcpMetricsShared};

pub async fn start_daemon(
    daemon_path: &Path,
    shared: Arc<XcpMetricsShared>,
) -> anyhow::Result<JoinHandle<()>> {
    let daemon_path = daemon_path.to_path_buf();

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
        hyper::Server::bind_unix(daemon_path)
            .expect("Unable to bind to socket")
            .serve(make_service)
            .await
            .unwrap();
    });

    Ok(server_task)
}
