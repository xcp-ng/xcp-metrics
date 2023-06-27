use tokio::{net::UnixStream, task::AbortHandle};
use xcp_metrics_common::xapi::{
    self,
    hyper::{
        self,
        service::{make_service_fn, service_fn},
        Body,
    },
    hyperlocal::UnixServerExt,
};

use crate::rpc::rpc;

pub struct XapiDaemon;

impl XapiDaemon {
    pub async fn new(daemon_name: &str) -> anyhow::Result<AbortHandle> {
        let socket_path = xapi::get_module_path(daemon_name);

        let make_service = make_service_fn(|socket: &UnixStream| {
            println!("{socket:?}");

            async move {
                anyhow::Ok(service_fn(move |request: hyper::Request<Body>| {
                    rpc::rpc_entrypoint(request)
                }))
            }
        });

        let server_task = tokio::task::spawn(async move {
            hyper::Server::bind_unix(socket_path)
                .expect("Unable to bind to port")
                .serve(make_service)
                .await
                .unwrap();
        });

        Ok(server_task.abort_handle())
    }
}
