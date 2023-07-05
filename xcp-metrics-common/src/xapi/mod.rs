use crate::rpc::XcpRpcMethod;
use std::{path::PathBuf, str::FromStr};

pub const XAPI_SOCKET_PATH: &str = "/var/lib/xcp";
pub const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

pub use hyper;
pub use hyperlocal;

pub fn get_module_path(name: &str) -> PathBuf {
    PathBuf::from_str(XAPI_SOCKET_PATH)
        .expect("Invalid XAPI_SOCKET_PATH")
        .join(name)
}

pub async fn send_xmlrpc_at<M: XcpRpcMethod>(
    name: &str,
    http_method: &str,
    rpc_method: &M,
    user_agent: &str,
) -> anyhow::Result<hyper::Response<hyper::Body>> {
    let module_uri = hyperlocal::Uri::new(get_module_path(name), "/");

    let mut rpc = String::default();
    rpc_method.write_xmlrpc(&mut rpc)?;

    let request = hyper::Request::builder()
        .uri(Into::<hyper::Uri>::into(module_uri))
        .method(http_method)
        .header("User-agent", user_agent)
        .header("content-length", rpc.len())
        .header("host", "localhost")
        .header("content-type", "text/xml")
        .body(rpc)?;

    Ok(hyper::Client::builder()
        .build(hyperlocal::UnixConnector)
        .request(request)
        .await?)
}

pub async fn send_jsonrpc_at<M: XcpRpcMethod>(
    name: &str,
    http_method: &str,
    rpc_method: &M,
    user_agent: &str,
) -> anyhow::Result<hyper::Response<hyper::Body>> {
    let module_uri = hyperlocal::Uri::new(get_module_path(name), "/");

    let mut rpc_buffer = vec![];
    rpc_method.write_jsonrpc(&mut rpc_buffer)?;

    let rpc = String::from_utf8(rpc_buffer)?;

    println!("{rpc:?}");

    let request = hyper::Request::builder()
        .uri(Into::<hyper::Uri>::into(module_uri))
        .method(http_method)
        .header("User-agent", user_agent)
        .header("content-length", rpc.len())
        .header("content-type", "application/json-rpc")
        .header("host", "localhost")
        .body(rpc)?;

    Ok(hyper::Client::builder()
        .build(hyperlocal::UnixConnector)
        .request(request)
        .await?)
}
