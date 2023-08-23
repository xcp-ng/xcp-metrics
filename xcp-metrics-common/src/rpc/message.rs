use hyper::{body::HttpBody, Body, Request, Response};
use serde::Serialize;

use crate::rpc::{parse_method_jsonrpc, parse_method_xmlrpc, XcpRpcMethod};

/// A RPC request that can be either in XML-RPC or JSON-RPC format.
#[derive(Debug, Clone)]
pub enum RpcRequest {
    XmlRpc(dxr::MethodCall),
    JsonRpc(jsonrpc_base::Request),
}

impl std::fmt::Display for RpcRequest {
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
    /// Deserialize the inner RPC method into a XCP RPC method.
    pub fn try_into_method<T: XcpRpcMethod>(self) -> Option<T> {
        match self {
            RpcRequest::XmlRpc(method) => T::try_from_xmlrpc(method),
            RpcRequest::JsonRpc(request) => T::try_from_jsonrpc(request),
        }
    }
}

impl RpcRequest {
    /// Get the name of the inner RPC request.
    pub fn get_name(&self) -> &str {
        match self {
            RpcRequest::XmlRpc(method_call) => method_call.name(),
            RpcRequest::JsonRpc(request) => &request.method,
        }
    }

    /// Parse a RPC request from a http request (either XML-RPC or JSON-RPC).
    pub async fn from_http(req: Request<Body>) -> Result<Self, anyhow::Error> {
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

                if let Some(Ok(bytes)) = req.into_body().data().await {
                    let buffer = String::from_utf8(bytes.to_vec())?;
                    parse_method_jsonrpc(&buffer).map(RpcRequest::JsonRpc)
                } else {
                    Err(anyhow::anyhow!("No content"))
                }
            }
            Some(Ok("text/xml")) | _ => {
                // XML
                if let Some(Ok(bytes)) = req.into_body().data().await {
                    let buffer = bytes.to_vec();
                    let str = String::from_utf8_lossy(&buffer);
                    //println!("RPC: XML:\n{str}");

                    parse_method_xmlrpc(&str).map(RpcRequest::XmlRpc)
                } else {
                    Err(anyhow::anyhow!("No content"))
                }
            }
        }
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

        Ok(builder.body(response.into_body()?)?)
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

    pub fn into_body(self) -> anyhow::Result<Body> {
        Ok(Body::from(match self {
            Self::XmlRpc(response) => quick_xml::se::to_string(&response)?,
            Self::JsonRpc(response) => serde_json::to_string(&response)?,
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

        Ok(builder.body(response.into_body()?)?)
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

    pub fn into_body(self) -> anyhow::Result<Body> {
        Ok(Body::from(match self {
            Self::XmlRpc(response) => quick_xml::se::to_string(&response)?,
            Self::JsonRpc(response) => serde_json::to_string(&response)?,
        }))
    }
}
