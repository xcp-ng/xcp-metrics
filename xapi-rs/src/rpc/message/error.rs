use std::fmt::Display;

use http::Response;
use serde::Serialize;

use super::{request::RpcRequest, xml::xml_to_string, RpcKind};

/// A RPC error that can be either in XML-RPC or JSON-RPC format.
#[derive(Clone, Debug)]
pub enum RpcError {
    XmlRpc(dxr::FaultResponse),
    JsonRpc(jsonrpc_base::Response),
}

impl RpcError {
    /// Parse the [RpcError].
    pub fn parse(data: &[u8], kind: RpcKind) -> anyhow::Result<Self> {
        Ok(match kind {
            RpcKind::XmlRpc => {
                let s = std::str::from_utf8(data)?;

                quick_xml::de::from_str(s).map(RpcError::XmlRpc)?
            }
            RpcKind::JsonRpc => serde_json::from_slice(data).map(RpcError::JsonRpc)?,
        })
    }

    /// Make a RPC error [`Response<Body>`] with `code`, `message` and a optional data for some [RpcRequest].
    pub fn respond_to<D: Serialize>(
        request: Option<&RpcRequest>,
        code: i32,
        message: &str,
        data: Option<D>,
    ) -> anyhow::Result<Response<String>> {
        let response = Self::make_error(request, code, message, data)?;

        Ok(Response::builder()
            .header("content-type", response.kind().to_mime())
            .body(response.to_body()?)?)
    }

    /// Make a [RpcError] with `code`, `message` and optional data for some [RpcRequest].
    pub fn make_error<D: Serialize>(
        request: Option<&RpcRequest>,
        code: i32,
        message: &str,
        data: Option<D>,
    ) -> anyhow::Result<Self> {
        Ok(match request {
            None | Some(RpcRequest::XmlRpc(_)) => {
                Self::XmlRpc(dxr::Fault::new(code, message.to_string()).into())
            }
            Some(RpcRequest::JsonRpc(jsonrpc_base::Request { id, .. })) => {
                Self::JsonRpc(jsonrpc_base::Response::ok(
                    id.clone(),
                    serde_json::to_value(jsonrpc_base::Error {
                        code,
                        message: message.to_string(),
                        data: data.and_then(|value| serde_json::to_value(value).ok()),
                    })?,
                ))
            }
        })
    }

    pub fn kind(&self) -> RpcKind {
        match self {
            Self::XmlRpc(_) => RpcKind::XmlRpc,
            Self::JsonRpc(_) => RpcKind::JsonRpc,
        }
    }

    /// Make a body from `self`.
    pub fn to_body(&self) -> anyhow::Result<String> {
        Ok(match self {
            Self::XmlRpc(fault) => xml_to_string(fault)?,
            Self::JsonRpc(response) => serde_json::to_string(response)?,
        })
    }
}

impl Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RpcError::XmlRpc(response) => match dxr::Fault::try_from(response.clone()) {
                Ok(fault) => write!(f, "XML-RPC: Fault {} ({})", fault.string(), fault.code()),
                Err(err) => {
                    write!(f, "XML-RPC: DXR error {err}")
                }
            },
            RpcError::JsonRpc(response) => match &response.error {
                Some(error) => write!(f, "{error} ({})", response.id),
                None => write!(f, "JSON-RPC: error id {}", response.id),
            },
        }
    }
}

impl std::error::Error for RpcError {}

/// Create a simple [RpcError].
pub fn make_rpc_error(code: i32, message: String, kind: RpcKind) -> RpcError {
    match kind {
        RpcKind::XmlRpc => RpcError::XmlRpc(dxr::Fault::new(code, message).into()),
        RpcKind::JsonRpc => RpcError::JsonRpc(jsonrpc_base::Response::ok(
            serde_json::Value::Null,
            serde_json::to_value(jsonrpc_base::Error {
                code,
                message,
                data: None,
            })
            .unwrap_or(serde_json::Value::Null),
        )),
    }
}
