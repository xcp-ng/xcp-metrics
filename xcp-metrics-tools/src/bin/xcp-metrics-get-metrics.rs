use tokio::io::{stdout, AsyncWriteExt};
use xcp_metrics_common::{
    xapi::{
        get_module_path,
        hyper::{self, body, Body},
        hyperlocal,
    },
    xmlrpc::dxr::MethodCall,
};

#[tokio::main]
async fn main() {
    let module_uri = hyperlocal::Uri::new(get_module_path("xcp-rrdd"), "/");

    let method = MethodCall::new("OpenMetrics".to_string(), vec![]);
    let rpc = quick_xml::se::to_string(&method).unwrap();

    eprintln!("Sent: {rpc}");

    let request = hyper::Request::builder()
        .uri(Into::<hyper::Uri>::into(module_uri))
        .method("POST")
        .header("User-agent", "xcp-metrics-test")
        .header("content-length", rpc.len())
        .header("host", "localhost")
        .body(Body::from(rpc))
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
