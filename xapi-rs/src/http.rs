use std::str::FromStr;

use crate::rpc::{
    message::{request::RpcRequest, RpcKind},
    XcpRpcMethod,
};
use reqwest::{Client, Method, Url};

/// Send a RPC request to the module `name`.
pub async fn send_rpc_to<M: XcpRpcMethod>(
    host: Url,
    http_method: &str,
    rpc_method: &M,
    user_agent: &str,
    kind: RpcKind,
) -> anyhow::Result<reqwest::Response> {
    Ok(Client::builder()
        .user_agent(user_agent)
        .build()?
        .request(Method::from_str(http_method)?, host)
        .header("content-type", kind.to_mime())
        .body(RpcRequest::new(rpc_method, kind)?.to_body()?)
        .send()
        .await?)
}
