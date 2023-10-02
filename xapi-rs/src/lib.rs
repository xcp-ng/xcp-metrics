//! XAPI utilities
pub mod rpc;
pub(crate) mod utils;

use std::path::{Path, PathBuf};

use crate::rpc::{message::RpcKind, write_method_jsonrpc, write_method_xmlrpc, XcpRpcMethod};
use hyper::{body, Body, Request, Response};

pub const XAPI_SOCKET_PATH: &str = "/var/lib/xcp";
pub const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

pub use hyper;
pub use hyperlocal;

/// Get the path of the socket of some module.
pub fn get_module_path(name: &str) -> PathBuf {
    Path::new(XAPI_SOCKET_PATH).join(name)
}

/// Send a XML-RPC request to the module `name`.
pub async fn send_rpc_to<M: XcpRpcMethod>(
    xapi_daemon_path: &Path,
    http_method: &str,
    rpc_method: &M,
    user_agent: &str,
    kind: RpcKind,
) -> anyhow::Result<Response<Body>> {
    let module_uri = hyperlocal::Uri::new(xapi_daemon_path, "/");

    let mut rpc = vec![];

    match kind {
        RpcKind::XmlRpc => write_method_xmlrpc(&mut rpc, rpc_method)?,
        RpcKind::JsonRpc => write_method_jsonrpc(&mut rpc, rpc_method)?,
    };

    let content_type = match kind {
        RpcKind::XmlRpc => "text/xml",
        RpcKind::JsonRpc => "application/json-rpc",
    };

    let request = Request::builder()
        .uri(hyper::Uri::from(module_uri))
        .method(http_method)
        .header("User-agent", user_agent)
        .header("content-length", rpc.len())
        .header("host", "localhost")
        .header("content-type", content_type)
        .body(body::Body::from(rpc))?;

    Ok(hyper::Client::builder()
        .build(hyperlocal::UnixConnector)
        .request(request)
        .await?)
}

/// Send a XML-RPC request to the module `name`.
pub async fn send_xmlrpc_to<M: XcpRpcMethod>(
    xapi_daemon_path: &Path,
    http_method: &str,
    rpc_method: &M,
    user_agent: &str,
) -> anyhow::Result<Response<Body>> {
    send_rpc_to(
        xapi_daemon_path,
        http_method,
        rpc_method,
        user_agent,
        RpcKind::XmlRpc,
    )
    .await
}

/// Send a JSON-RPC request to the module `name`.
pub async fn send_jsonrpc_to<M: XcpRpcMethod>(
    xapi_daemon_path: &Path,
    http_method: &str,
    rpc_method: &M,
    user_agent: &str,
) -> anyhow::Result<Response<Body>> {
    send_rpc_to(
        xapi_daemon_path,
        http_method,
        rpc_method,
        user_agent,
        RpcKind::JsonRpc,
    )
    .await
}
