//! XAPI utilities
use crate::rpc::{write_method_jsonrpc, write_method_xmlrpc, XcpRpcMethod};
use std::{path::PathBuf, str::FromStr};

pub const XAPI_SOCKET_PATH: &str = "/var/lib/xcp";
pub const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

pub use hyper;
use hyper::body;
pub use hyperlocal;

/// Get the path of the socket of some module.
pub fn get_module_path(name: &str) -> PathBuf {
    PathBuf::from_str(XAPI_SOCKET_PATH)
        .expect("Invalid XAPI_SOCKET_PATH")
        .join(name)
}

/// Send a XML-RPC request to the module `name`.
pub async fn send_xmlrpc_at<M: XcpRpcMethod>(
    name: &str,
    http_method: &str,
    rpc_method: &M,
    user_agent: &str,
) -> anyhow::Result<hyper::Response<hyper::Body>> {
    let module_uri = hyperlocal::Uri::new(get_module_path(name), "/");

    let mut rpc = vec![];
    write_method_xmlrpc(&mut rpc, rpc_method)?;

    let request = hyper::Request::builder()
        .uri(hyper::Uri::from(module_uri))
        .method(http_method)
        .header("User-agent", user_agent)
        .header("content-length", rpc.len())
        .header("host", "localhost")
        .header("content-type", "text/xml")
        .body(body::Body::from(rpc))?;

    Ok(hyper::Client::builder()
        .build(hyperlocal::UnixConnector)
        .request(request)
        .await?)
}

/// Send a JSON-RPC request to the module `name`.
pub async fn send_jsonrpc_at<M: XcpRpcMethod>(
    name: &str,
    http_method: &str,
    rpc_method: &M,
    user_agent: &str,
) -> anyhow::Result<hyper::Response<hyper::Body>> {
    let module_uri = hyperlocal::Uri::new(get_module_path(name), "/");

    let mut rpc = vec![];
    write_method_jsonrpc(&mut rpc, rpc_method)?;

    let request = hyper::Request::builder()
        .uri(hyper::Uri::from(module_uri))
        .method(http_method)
        .header("User-agent", user_agent)
        .header("content-length", rpc.len())
        .header("content-type", "application/json-rpc")
        .header("host", "localhost")
        .body(body::Body::from(rpc))?;

    Ok(hyper::Client::builder()
        .build(hyperlocal::UnixConnector)
        .request(request)
        .await?)
}
