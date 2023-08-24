//! RPC utilities
pub mod message;
pub mod methods;
pub mod response;
pub(super) mod xml;

#[cfg(test)]
mod test;

use std::io::Write;

use dxr::{TryFromValue, TryToValue};
use serde::{de::DeserializeOwned, Serialize};

use self::message::{RpcKind, RpcRequest};

#[macro_export]
macro_rules! rpc_method {
    ($struct:ty, $name:stmt) => {
        impl $crate::rpc::XcpRpcMethodNamed for $struct {
            fn get_method_name() -> &'static str {
                $name
            }
        }
    };
}

pub trait XcpRpcMethodNamed {
    fn get_method_name() -> &'static str;
}

pub trait XcpRpcMethod: Sized {
    fn to_xmlrpc(&self) -> anyhow::Result<dxr::MethodCall>;
    fn to_jsonrpc(&self) -> anyhow::Result<jsonrpc_base::Request>;

    fn try_from_xmlrpc(call: dxr::MethodCall) -> Option<Self>;
    fn try_from_jsonrpc(request: jsonrpc_base::Request) -> Option<Self>;
}

impl<M> XcpRpcMethod for M
where
    M: TryToValue + TryFromValue + XcpRpcMethodNamed + Serialize + DeserializeOwned,
{
    fn to_xmlrpc(&self) -> anyhow::Result<dxr::MethodCall> {
        Ok(dxr::MethodCall::new(
            M::get_method_name().into(),
            vec![self.try_to_value()?],
        ))
    }

    fn to_jsonrpc(&self) -> anyhow::Result<jsonrpc_base::Request> {
        let id = serde_json::Value::String(uuid::Uuid::new_v4().as_hyphenated().to_string());

        Ok(jsonrpc_base::Request {
            jsonrpc: "2.0".into(),
            id,
            method: Self::get_method_name().into(),
            params: Some(serde_json::to_value(self)?),
        })
    }

    fn try_from_xmlrpc(method: dxr::MethodCall) -> Option<Self> {
        if method.name() == M::get_method_name() {
            M::try_from_value(method.params().first()?).ok()
        } else {
            None
        }
    }

    fn try_from_jsonrpc(request: jsonrpc_base::Request) -> Option<Self> {
        if request.method == M::get_method_name() {
            serde_json::from_value(request.params?).ok()
        } else {
            None
        }
    }
}

pub fn write_method_jsonrpc<W: Write, M: XcpRpcMethod>(
    writer: &mut W,
    method: &M,
) -> anyhow::Result<()> {
    RpcRequest::new(method, RpcKind::JsonRpc)?.write(writer)
}

pub fn write_method_xmlrpc<W: Write, M: XcpRpcMethod>(
    writer: &mut W,
    method: &M,
) -> anyhow::Result<()> {
    RpcRequest::new(method, RpcKind::XmlRpc)?.write(writer)
}

pub fn parse_method_jsonrpc<M: XcpRpcMethod>(data: &[u8]) -> anyhow::Result<M> {
    RpcRequest::parse(data, RpcKind::JsonRpc)?
        .try_into_method()
        .ok_or(anyhow::anyhow!("Readed method doesn't match"))
}

pub fn parse_method_xmlrpc<M: XcpRpcMethod>(data: &[u8]) -> anyhow::Result<M> {
    RpcRequest::parse(data, RpcKind::XmlRpc)?
        .try_into_method()
        .ok_or(anyhow::anyhow!("Readed method doesn't match"))
}
