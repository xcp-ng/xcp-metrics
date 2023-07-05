use crate::{
    rpc::{parse_method_jsonrpc, parse_method_xmlrpc, XcpRpcMethod},
    xapi::hyper::{self, body::HttpBody, Body},
};

#[derive(Clone, Debug)]
pub enum RpcRequest {
    XmlRpc(dxr::MethodCall),
    JsonRpc(jsonrpc_base::Request),
}

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

                    parse_method_xmlrpc(&str).map(RpcRequest::XmlRpc)
                } else {
                    Err(anyhow::anyhow!("No content"))
                }
            }
        }
    }
}
