use hyper::Response;
use serde::Serialize;

use crate::{
    rpc::{parse_method_jsonrpc, parse_method_xmlrpc, XcpRpcMethod},
    xapi::hyper::{self, body::HttpBody, Body},
};

#[derive(Clone, Debug)]
pub enum RpcRequest {
    XmlRpc(dxr::MethodCall),
    JsonRpc(jsonrpc_base::Request),
}

// TODO: Make a structure that implements Error and that can convert to RpcError ?

impl RpcRequest {
    pub fn try_into_method<T: XcpRpcMethod>(self) -> Option<T> {
        match self {
            RpcRequest::XmlRpc(method) => T::try_from_xmlrpc(method),
            RpcRequest::JsonRpc(request) => T::try_from_jsonrpc(request),
        }
    }
}

impl RpcRequest {
    pub fn get_name(&self) -> &str {
        match self {
            RpcRequest::XmlRpc(method_call) => method_call.name(),
            RpcRequest::JsonRpc(request) => &request.method,
        }
    }

    pub async fn from_http(req: hyper::Request<Body>) -> Result<Self, anyhow::Error> {
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
                    let buffer = bytes.to_vec();
                    parse_method_jsonrpc(&mut buffer.as_slice()).map(RpcRequest::JsonRpc)
                } else {
                    Err(anyhow::anyhow!("No content"))
                }
            }
            Some(Ok("text/xml")) | _ => {
                // XML
                if let Some(Ok(bytes)) = req.into_body().data().await {
                    let buffer = bytes.to_vec();
                    let str = String::from_utf8_lossy(&buffer);
                    println!("RPC: XML:\n{str}");

                    parse_method_xmlrpc(&str).map(RpcRequest::XmlRpc)
                } else {
                    Err(anyhow::anyhow!("No content"))
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum RpcResponse {
    XmlRpc(dxr::MethodResponse),
    JsonRpc(jsonrpc_base::Response),
}

impl RpcResponse {
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

    pub fn into_body(self) -> anyhow::Result<hyper::Body> {
        Ok(hyper::Body::from(match self {
            Self::XmlRpc(response) => quick_xml::se::to_string(&response)?,
            Self::JsonRpc(response) => serde_json::to_string(&response)?,
        }))
    }
}

#[derive(Clone, Debug)]
pub enum RpcError {
    XmlRpc(dxr::FaultResponse),
    JsonRpc(jsonrpc_base::Response),
}

impl RpcError {
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
                        data: data.map(|value| serde_json::to_value(value).ok()).flatten(),
                    })?,
                ))
            }
        })
    }

    pub fn into_body(self) -> anyhow::Result<hyper::Body> {
        Ok(hyper::Body::from(match self {
            Self::XmlRpc(response) => quick_xml::se::to_string(&response)?,
            Self::JsonRpc(response) => serde_json::to_string(&response)?,
        }))
    }
}
