use std::path::{Path, PathBuf};

use crate::rpc::{
    message::{request::RpcRequest, RpcKind},
    XcpRpcMethod,
};

use hyper::{body::Incoming, client::conn::http1, Request, Response};
use hyper_util::rt::TokioIo;
use tokio::{net::UnixStream, task};

pub const XAPI_SOCKET_PATH: &str = "/var/lib/xcp";
pub const METRICS_SHM_PATH: &str = "/dev/shm/metrics/";

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
) -> anyhow::Result<Response<Incoming>> {
    let rpc_body = RpcRequest::new(rpc_method, kind)?.to_body()?;

    let request = Request::builder()
        .uri("/")
        .method(http_method)
        .header("user-agent", user_agent)
        .header("host", "localhost")
        .header("content-type", kind.to_mime())
        .body(rpc_body)?;

    let stream = UnixStream::connect(xapi_daemon_path).await?;

    let io = TokioIo::new(stream);

    let (mut sender, connection) = http1::handshake(io).await?;

    task::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("send_rpc_to: {e}")
        }
    });

    Ok(sender.send_request(request).await?)
}
