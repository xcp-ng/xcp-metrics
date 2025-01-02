use std::fmt::Display;

use super::{xml::xml_to_string, RpcKind};
use crate::rpc::XcpRpcMethod;

/// A RPC request that can be either in XML-RPC or JSON-RPC format.
#[derive(Debug, Clone)]
pub enum RpcRequest {
    XmlRpc(dxr::MethodCall),
    JsonRpc(jsonrpc_base::Request),
}

impl Display for RpcRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Kind: {}\tMethod: {}",
            match self {
                RpcRequest::XmlRpc(_) => "XML-RPC",
                RpcRequest::JsonRpc(_) => "JSON-RPC",
            },
            self.get_name(),
        )
    }
}

impl RpcRequest {
    /// Create a new [RpcRequest] from a [XcpRpcMethod].
    /// Fails if the method cannot be converted to request.
    pub fn new(method: &impl XcpRpcMethod, kind: RpcKind) -> anyhow::Result<Self> {
        Ok(match kind {
            RpcKind::XmlRpc => Self::XmlRpc(method.to_xmlrpc()?),
            RpcKind::JsonRpc => Self::JsonRpc(method.to_jsonrpc()?),
        })
    }

    /// Parse a [RpcRequest].
    pub fn parse(data: &[u8], kind: RpcKind) -> anyhow::Result<Self> {
        Ok(match kind {
            RpcKind::XmlRpc => {
                let s = std::str::from_utf8(data)?;

                quick_xml::de::from_str(s).map(RpcRequest::XmlRpc)?
            }
            RpcKind::JsonRpc => serde_json::from_slice(data).map(RpcRequest::JsonRpc)?,
        })
    }

    /// Deserialize the inner request into a [XcpRpcMethod].
    /// Fails if method doesn't match the request.
    pub fn try_into_method<T: XcpRpcMethod>(self) -> Option<T> {
        match self {
            Self::XmlRpc(method) => T::try_from_xmlrpc(method),
            Self::JsonRpc(request) => T::try_from_jsonrpc(request),
        }
    }

    /// Get the name of the inner RPC request.
    pub fn get_name(&self) -> &str {
        match self {
            Self::XmlRpc(method_call) => method_call.name(),
            Self::JsonRpc(request) => &request.method,
        }
    }

    /// Make a rpc body from `self`.
    pub fn to_body(&self) -> anyhow::Result<String> {
        Ok(match self {
            Self::XmlRpc(response) => xml_to_string(response)?,
            Self::JsonRpc(response) => serde_json::to_string(response)?,
        })
    }
}
