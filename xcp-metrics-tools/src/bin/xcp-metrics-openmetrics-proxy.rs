use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use hyper::{
    server::{conn::AddrStream, Server},
    service::{make_service_fn, service_fn},
};

use xcp_metrics_common::{rpc::methods::OpenMetricsMethod, xapi};

#[tokio::main]
async fn main() {
    let Some(port): Option<u16> = env::args().nth(1).and_then(|port_arg| port_arg.parse().ok()) else {
        eprintln!("Usage: xcp-metrics-openmetrics-proxy <port>");
        return
    };

    let service_fn = make_service_fn(|addr: &AddrStream| {
        println!("Handling request {:?}", addr);

        async {
            anyhow::Ok(service_fn(|_req| async {
                xapi::send_jsonrpc_at(
                    "xcp-rrdd",
                    "POST",
                    &OpenMetricsMethod::default(),
                    "xcp-metrics-openmetrics-proxy",
                )
                .await
            }))
        }
    });

    let server =
        Server::bind(&SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port)).serve(service_fn);

    server.await.expect("Proxy server failure");
}
