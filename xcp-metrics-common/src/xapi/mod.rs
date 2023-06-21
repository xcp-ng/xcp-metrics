use hyper::{body::Incoming, client::conn::http1};
use tokio::net::UnixStream;

use crate::xmlrpc::XcpRpcMethod;
use std::{path::PathBuf, str::FromStr};

const XAPI_SOCKET_PATH: &str = "/var/lib/xcp";

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
) -> anyhow::Result<hyper::Response<Incoming>> {
    let module_path = get_module_path(name);
    let stream = UnixStream::connect(module_path).await?;

    let mut rpc = String::default();
    rpc_method.write_xmlrpc(&mut rpc)?;

    let request = hyper::Request::builder()
        .method(http_method)
        .header("User-agent", user_agent)
        .header("content-length", rpc.len())
        .header("host", "localhost")
        .body(rpc)?;

    let (mut send, _) = http1::handshake(stream).await?;

    let response = send.send_request(request).await?;
    Ok(response)
}
