//! Various utilities to make and parse RPC requests.
pub mod error;
pub mod request;
pub mod response;

#[cfg(test)]
mod test;

pub(self) mod xml;

use std::{fmt::Display, str::FromStr};

use http::{Request, Response};
use http_body::Body;
use http_body_util::BodyExt;

use self::{
    error::{make_rpc_error, RpcError},
    request::RpcRequest,
    response::RpcResponse,
};

use crate::rpc::XcpRpcMethod;

/// A kind of RPC request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RpcKind {
    #[default]
    XmlRpc,
    JsonRpc,
}

impl Display for RpcKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RpcKind::XmlRpc => "XML-RPC",
                RpcKind::JsonRpc => "JSON-RPC",
            }
        )
    }
}

impl FromStr for RpcKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "xml" | "xmlrpc" | "xml-rpc" => Ok(Self::XmlRpc),
            "json" | "jsonrpc" | "json-rpc" => Ok(Self::JsonRpc),
            _ => Err("Unknown RPC format".to_string()),
        }
    }
}

impl RpcKind {
    pub fn from_mime(mime: &str) -> Self {
        match mime {
            // JSON
            "application/json" | "application/json-rpc" | "application/jsonrequest" => {
                Self::JsonRpc
            }
            // XML
            "text/xml" | _ => Self::XmlRpc,
        }
    }

    pub fn to_mime(&self) -> &str {
        match self {
            Self::XmlRpc => "text/xml",
            Self::JsonRpc => "application/json",
        }
    }

    fn from_headers(headers: &http::HeaderMap) -> RpcKind {
        headers
            .get("content-type")
            .map(|header| header.to_str().ok())
            .flatten()
            .map_or(RpcKind::XmlRpc, RpcKind::from_mime)
    }
}

/// Parse either a [RpcResponse] or a [RpcError].
/// Convert all internal errors to [RpcError].
pub fn parse_rpc_response(data: &[u8], kind: RpcKind) -> Result<RpcResponse, RpcError> {
    // Try to parse as RpcResponse.
    RpcResponse::parse(data, kind)
        // Retry as RpcError
        .map_err(|_| {
            RpcError::parse(data, kind).unwrap_or_else(|err| {
                // Other parse error
                make_rpc_error(
                    jsonrpc_base::Error::PARSE_ERROR,
                    format!("Unable to parse RPC response: {err}"),
                    kind,
                )
            })
        })
}

/// Parse either a [RpcResponse] or a [RpcError] from [`Response<Body>`].
/// Convert all internal errors to [RpcError].
pub async fn parse_http_response<B: Body>(response: Response<B>) -> Result<RpcResponse, RpcError>
where
    <B as Body>::Error: std::fmt::Display,
{
    // TODO: Handle chained requests ?
    let (parts, body) = response.into_parts();
    let kind = RpcKind::from_headers(&parts.headers);

    let data = body
        .collect()
        .await
        .map_err(|e| {
            make_rpc_error(
                -32603, /* Internal error */
                format!("Internal error: {e}"),
                kind,
            )
        })?
        .to_bytes();

    parse_rpc_response(&data, kind)
}

pub async fn parse_http_request<B: Body>(request: Request<B>) -> anyhow::Result<RpcRequest>
where
    <B as Body>::Error: std::fmt::Display,
{
    let (parts, body) = request.into_parts();
    let kind = RpcKind::from_headers(&parts.headers);

    let data = body
        .collect()
        .await
        .map_err(|e| anyhow::format_err!("{e}"))?
        .to_bytes();

    Ok(RpcRequest::parse(&data, kind)?)
}

/// Parse a [XcpRpcMethod] from raw JSON-RPC.
pub fn parse_method_jsonrpc<M: XcpRpcMethod>(data: &[u8]) -> anyhow::Result<M> {
    RpcRequest::parse(data, RpcKind::JsonRpc)?
        .try_into_method()
        .ok_or(anyhow::anyhow!("Readed method doesn't match"))
}

/// Parse [XcpRpcMethod] from raw XML-RPC.
pub fn parse_method_xmlrpc<M: XcpRpcMethod>(data: &[u8]) -> anyhow::Result<M> {
    RpcRequest::parse(data, RpcKind::XmlRpc)?
        .try_into_method()
        .ok_or(anyhow::anyhow!("Readed method doesn't match"))
}
