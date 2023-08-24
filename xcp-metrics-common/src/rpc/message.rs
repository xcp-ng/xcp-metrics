use std::{fmt::Display, io::Write, str::FromStr};

use hyper::{body::HttpBody, Body, Request, Response};
use serde::Serialize;

use super::xml::write_xml;
use crate::rpc::XcpRpcMethod;

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
            "xml" => Ok(Self::XmlRpc),
            "json" => Ok(Self::JsonRpc),
            _ => Err("Unknown RPC format".to_string()),
        }
    }
}

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

// TODO: Make a structure that implements Error and that can convert to RpcError ?

impl RpcRequest {
    pub fn new<M: XcpRpcMethod>(method: &M, kind: RpcKind) -> anyhow::Result<Self> {
        Ok(match kind {
            RpcKind::XmlRpc => Self::XmlRpc(method.to_xmlrpc()?),
            RpcKind::JsonRpc => Self::JsonRpc(method.to_jsonrpc()?),
        })
    }

    /// Parse the RPC request.
    pub fn parse(data: &[u8], kind: RpcKind) -> anyhow::Result<Self> {
        Ok(match kind {
            RpcKind::XmlRpc => {
                let s = std::str::from_utf8(data)?;

                quick_xml::de::from_str(s).map(RpcRequest::XmlRpc)?
            }
            RpcKind::JsonRpc => serde_json::from_slice(data).map(RpcRequest::JsonRpc)?,
        })
    }

    /// Deserialize the inner RPC method into a XCP RPC method.
    pub fn try_into_method<T: XcpRpcMethod>(self) -> Option<T> {
        match self {
            RpcRequest::XmlRpc(method) => T::try_from_xmlrpc(method),
            RpcRequest::JsonRpc(request) => T::try_from_jsonrpc(request),
        }
    }

    /// Get the name of the inner RPC request.
    pub fn get_name(&self) -> &str {
        match self {
            RpcRequest::XmlRpc(method_call) => method_call.name(),
            RpcRequest::JsonRpc(request) => &request.method,
        }
    }

    /// Write the serialized RPC request to `writer`.
    pub fn write<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        Ok(match self {
            RpcRequest::XmlRpc(method) => write_xml(writer, method)?,
            RpcRequest::JsonRpc(request) => serde_json::to_writer(writer, request)?,
        })
    }

    /// Parse a RPC request from a http request (either XML-RPC or JSON-RPC).
    pub async fn from_http(req: Request<Body>) -> anyhow::Result<Self> {
        Ok(
            match req
                .headers()
                .get("content-type")
                .map(|header| header.to_str())
            {
                Some(Ok("application/json"))
                | Some(Ok("application/json-rpc"))
                | Some(Ok("application/jsonrequest")) => {
                    // JSON
                    // TODO: Handle chained requests ?
                    Self::from_body(&mut req.into_body(), RpcKind::JsonRpc).await?
                }
                Some(Ok("text/xml")) | _ => {
                    // XML
                    Self::from_body(&mut req.into_body(), RpcKind::XmlRpc).await?
                }
            },
        )
    }

    /// Parse a RPC request from a http body (either XML-RPC or JSON-RPC).
    pub async fn from_body(body: &mut Body, kind: RpcKind) -> anyhow::Result<Self> {
        if let Some(Ok(bytes)) = body.data().await {
            Self::parse(bytes.as_ref(), kind)
        } else {
            Err(anyhow::anyhow!("No content"))
        }
    }

    pub fn to_body(&self) -> anyhow::Result<Body> {
        Ok(Body::from(match self {
            Self::XmlRpc(response) => quick_xml::se::to_string(response)?,
            Self::JsonRpc(response) => serde_json::to_string(response)?,
        }))
    }
}

/// A RPC response that can be either in XML-RPC or JSON-RPC format.
#[derive(Clone, Debug)]
pub enum RpcResponse {
    XmlRpc(dxr::MethodResponse),
    JsonRpc(jsonrpc_base::Response),
}

impl RpcResponse {
    /// Make a HTTP RPC response with `message` for some RPC request.
    pub fn respond_to<M>(request: &RpcRequest, message: M) -> anyhow::Result<Response<Body>>
    where
        M: Serialize + dxr::TryToValue,
    {
        let mut builder = Response::builder();
        let response = Self::make_response(request, message)?;

        builder = match response {
            Self::XmlRpc(_) => builder.header("content-type", "text/xml"),
            Self::JsonRpc(_) => builder.header("content-type", "application/json"),
        };

        Ok(builder.body(response.to_body()?)?)
    }

    /// Make a RPC response object with `message` for some RPC request.
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

    pub fn to_body(&self) -> anyhow::Result<Body> {
        Ok(Body::from(match self {
            Self::XmlRpc(response) => quick_xml::se::to_string(response)?,
            Self::JsonRpc(response) => serde_json::to_string(response)?,
        }))
    }
}

/// A RPC error that can be either in XML-RPC or JSON-RPC format.
#[derive(Clone, Debug)]
pub enum RpcError {
    XmlRpc(dxr::FaultResponse),
    JsonRpc(jsonrpc_base::Response),
}

impl RpcError {
    /// Make a HTTP RPC error with `code`, `message` and optional data for some RPC request.
    pub fn respond_to<D: Serialize>(
        request: Option<&RpcRequest>,
        code: i32,
        message: &str,
        data: Option<D>,
    ) -> anyhow::Result<Response<Body>> {
        let mut builder = Response::builder();
        let response = Self::make_error(request, code, message, data)?;

        builder = match response {
            Self::XmlRpc(_) => builder.header("content-type", "text/xml"),
            Self::JsonRpc(_) => builder.header("content-type", "application/json"),
        };

        Ok(builder.body(response.to_body()?)?)
    }

    /// Make a RPC error object with `code`, `message` and optional data for some RPC request.
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

    pub fn to_body(&self) -> anyhow::Result<Body> {
        Ok(Body::from(match self {
            Self::XmlRpc(response) => quick_xml::se::to_string(response)?,
            Self::JsonRpc(response) => serde_json::to_string(response)?,
        }))
    }
}
