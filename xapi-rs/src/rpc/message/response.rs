use http::Response;
use serde::Serialize;

use super::{request::RpcRequest, xml::xml_to_string, RpcKind};

/// A RPC response that can be either in XML-RPC or JSON-RPC format.
#[derive(Clone, Debug)]
pub enum RpcResponse {
    XmlRpc(dxr::MethodResponse),
    JsonRpc(jsonrpc_base::Response),
}

impl RpcResponse {
    /// Make a [RpcResponse] with `message` that respond to a [RpcRequest].
    /// Fails if the method cannot be converted to response.
    pub fn make_response<M>(request: &RpcRequest, message: M) -> anyhow::Result<Self>
    where
        M: Serialize + dxr::TryToValue,
    {
        Ok(match request {
            RpcRequest::XmlRpc(_) => {
                Self::XmlRpc(dxr::MethodResponse::new(message.try_to_value()?))
            }
            RpcRequest::JsonRpc(jsonrpc_base::Request { id, .. }) => Self::JsonRpc(
                jsonrpc_base::Response::ok(id.clone(), serde_json::to_value(message)?),
            ),
        })
    }

    /// Make a [`Response<Body>`] with a `message` that respond to a [RpcRequest].
    /// Fails if the method cannot be converted to response.
    pub fn respond_to<M>(request: &RpcRequest, message: M) -> anyhow::Result<Response<String>>
    where
        M: Serialize + dxr::TryToValue,
    {
        let response = Self::make_response(request, message)?;

        Ok(Response::builder()
            .header("content-type", response.kind().to_mime())
            .body(response.to_body()?)?)
    }

    /// Parse the [RpcResponse].
    pub fn parse(data: &[u8], kind: RpcKind) -> anyhow::Result<Self> {
        Ok(match kind {
            RpcKind::XmlRpc => {
                let s = std::str::from_utf8(data)?;

                quick_xml::de::from_str(s).map(RpcResponse::XmlRpc)?
            }
            RpcKind::JsonRpc => serde_json::from_slice(data).map(RpcResponse::JsonRpc)?,
        })
    }

    fn kind(&self) -> RpcKind {
        match self {
            Self::XmlRpc(_) => RpcKind::XmlRpc,
            Self::JsonRpc(_) => RpcKind::JsonRpc,
        }
    }

    /// Make a [String] from `self`.
    pub fn to_body(&self) -> anyhow::Result<String> {
        Ok(match self {
            Self::XmlRpc(response) => xml_to_string(response)?,
            Self::JsonRpc(response) => serde_json::to_string(response)?,
        })
    }
}
