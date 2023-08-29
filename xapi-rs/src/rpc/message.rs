//! Various utilities to make and parse RPC requests.
use std::{fmt::Display, io::Write, str::FromStr};

use hyper::{body::HttpBody, Body, Request, Response};
use serde::Serialize;

use super::xml::{write_xml, xml_to_string};
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
            "xml" | "xmlrpc" => Ok(Self::XmlRpc),
            "json" | "jsonrpc" => Ok(Self::JsonRpc),
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

impl RpcRequest {
    /// Create a new [RpcRequest] from a method.
    /// Fails if the method cannot be converted to request.
    pub fn new<M: XcpRpcMethod>(method: &M, kind: RpcKind) -> anyhow::Result<Self> {
        Ok(match kind {
            RpcKind::XmlRpc => Self::XmlRpc(method.to_xmlrpc()?),
            RpcKind::JsonRpc => Self::JsonRpc(method.to_jsonrpc()?),
        })
    }

    /// Parse the [RpcRequest].
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

    /// Write the serialized [RpcRequest] to `writer`.
    pub fn write<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        match self {
            RpcRequest::XmlRpc(method) => write_xml(writer, method)?,
            RpcRequest::JsonRpc(request) => serde_json::to_writer(writer, request)?,
        }

        Ok(())
    }

    /// Parse a [RpcRequest] from a http request (either XML-RPC or JSON-RPC).
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

    /// Parse a [RpcRequest] from a [Body].
    pub async fn from_body(body: &mut Body, kind: RpcKind) -> anyhow::Result<Self> {
        if let Some(Ok(bytes)) = body.data().await {
            Self::parse(bytes.as_ref(), kind)
        } else {
            Err(anyhow::anyhow!("No content"))
        }
    }

    /// Make a [Body] from `self`.
    pub fn to_body(&self) -> anyhow::Result<Body> {
        Ok(Body::from(match self {
            Self::XmlRpc(response) => xml_to_string(response)?,
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

    /// Make a [`Response<Body>`] with a `message` that respond to a [RpcRequest].
    /// Fails if the method cannot be converted to response.
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

    /// Parse a [RpcRequest] from a [Body].
    pub async fn from_body(body: &mut Body, kind: RpcKind) -> anyhow::Result<Self> {
        if let Some(Ok(bytes)) = body.data().await {
            Self::parse(bytes.as_ref(), kind)
        } else {
            Err(anyhow::anyhow!("No content"))
        }
    }

    /// Make a [Body] from `self`.
    pub fn to_body(&self) -> anyhow::Result<Body> {
        Ok(Body::from(match self {
            Self::XmlRpc(response) => xml_to_string(response)?,
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
    ) -> anyhow::Result<Response<Body>> {
        let mut builder = Response::builder();
        let response = Self::make_error(request, code, message, data)?;

        builder = match response {
            Self::XmlRpc(_) => builder.header("content-type", "text/xml"),
            Self::JsonRpc(_) => builder.header("content-type", "application/json"),
        };

        Ok(builder.body(response.to_body()?)?)
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

    /// Parse a [RpcError] from a [Body].
    pub async fn from_body(body: &mut Body, kind: RpcKind) -> anyhow::Result<Self> {
        if let Some(Ok(bytes)) = body.data().await {
            Self::parse(bytes.as_ref(), kind)
        } else {
            Err(anyhow::anyhow!("No content"))
        }
    }

    /// Make a [Body] from `self`.
    pub fn to_body(&self) -> anyhow::Result<Body> {
        Ok(Body::from(match self {
            Self::XmlRpc(fault) => xml_to_string(fault)?,
            Self::JsonRpc(response) => serde_json::to_string(response)?,
        }))
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

/// Parse either a [RpcResponse] or a [RpcError] from [`Request<Body>`].
/// Convert all internal errors to [RpcError].
pub async fn parse_http_response(response: Response<Body>) -> Result<RpcResponse, RpcError> {
    // TODO: Handle chained requests ?
    let (parts, mut body) = response.into_parts();

    let kind = match parts
        .headers
        .get("content-type")
        .map(|header| header.to_str())
    {
        // JSON
        Some(Ok("application/json"))
        | Some(Ok("application/json-rpc"))
        | Some(Ok("application/jsonrequest")) => RpcKind::JsonRpc,
        // XML
        Some(Ok("text/xml")) => RpcKind::XmlRpc,

        _ => {
            return Err(make_rpc_error(
                jsonrpc_base::Error::INVALID_REQUEST,
                "Invalid or undefined content-type".to_string(),
                RpcKind::XmlRpc,
            ));
        }
    };

    let data = match body.data().await {
        Some(Ok(bytes)) => bytes.to_vec(),
        Some(Err(e)) => {
            return Err(make_rpc_error(
                -32603, /* Internal error */
                format!("Internal error: {e}"),
                RpcKind::XmlRpc,
            ));
        }
        None => {
            return Err(make_rpc_error(
                jsonrpc_base::Error::INVALID_REQUEST,
                "No body provided".to_string(),
                RpcKind::XmlRpc,
            ))
        }
    };

    parse_rpc_response(&data, kind)
}
