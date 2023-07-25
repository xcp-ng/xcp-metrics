use tokio::io::{stdout, AsyncWriteExt};
use xcp_metrics_common::{
    rpc::{methods::OpenMetricsMethod, XcpRpcMethod},
    xapi::{
        get_module_path,
        hyper::{self, body, Body},
        hyperlocal,
    },
};

#[tokio::main]
async fn main() {
    let module_uri = hyperlocal::Uri::new(get_module_path("xcp-metrics"), "/");

    /*
    let method = MethodCall::new("OpenMetrics".to_string(), vec![]);
    let rpc = quick_xml::se::to_string(&method).unwrap();
    */

    let mut rpc_buffer = vec![];
    OpenMetricsMethod::default()
        .write_jsonrpc(&mut rpc_buffer)
        .unwrap();

    eprintln!("Sent: {}", String::from_utf8_lossy(&rpc_buffer));

    let request = hyper::Request::builder()
        .uri(Into::<hyper::Uri>::into(module_uri))
        .method("POST")
        .header("User-agent", "xcp-metrics-test")
        .header("content-length", rpc_buffer.len())
        .header("content-type", "application/json-rpc")
        .header("host", "localhost")
        .body(Body::from(rpc_buffer))
        .unwrap();

    let response = hyper::Client::builder()
        .build(hyperlocal::UnixConnector)
        .request(request)
        .await;

    eprintln!("{response:#?}");

    let response = response.unwrap();
    let data = body::to_bytes(response.into_body()).await.unwrap();

    stdout().write_all(&data).await.unwrap();
}
