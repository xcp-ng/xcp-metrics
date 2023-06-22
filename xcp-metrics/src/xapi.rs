use std::future::Future;

use tokio::net::UnixStream;
use xcp_metrics_common::xapi::{
    self,
    hyper::{
        self,
        service::{make_service_fn, service_fn},
        Body, Request, Response,
    },
    hyperlocal::UnixServerExt,
};

pub struct XapiDaemon {}

type HookFn = Box<dyn Fn(Request<Body>) -> Option<Box<dyn Future<Output = Response<Body>>>>>;

pub struct XapiDaemonInternal {
    hooks: Vec<HookFn>,
}

impl Default for XapiDaemonInternal {
    fn default() -> Self {
        Self { hooks: vec![] }
    }
}

impl XapiDaemon {
    async fn new(daemon_name: &str) -> anyhow::Result<Self> {
        let socket_path = xapi::get_module_path(daemon_name);

        let make_service = make_service_fn(|socket: &UnixStream| {
            println!("{socket:?}");

            async move { anyhow::Ok(service_fn(move |request: hyper::Request<Body>| ())) }
        });

        let server_task = tokio::task::spawn(async move {
            hyper::Server::bind_unix(socket_path)
                .expect("Unable to bind to port")
                .serve(make_service)
                .await
                .unwrap();
        });

        Ok(server_task)
    }
}
