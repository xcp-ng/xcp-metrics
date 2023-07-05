pub mod methods;

use dxr::{TryFromValue, TryToValue};

pub use dxr;
use serde::{de::DeserializeOwned, Serialize};

#[macro_export]
macro_rules! rpc_method {
    ($struct:ty, $name:stmt) => {
        impl crate::rpc::XcpRpcMethodNamed for $struct {
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
    fn write_xmlrpc<W: std::fmt::Write>(&self, w: &mut W) -> anyhow::Result<()>;
    fn write_jsonrpc<W: std::io::Write>(&self, w: &mut W) -> anyhow::Result<()>;

    fn try_from_xmlrpc(call: dxr::MethodCall) -> Option<Self>;
    fn try_from_jsonrpc(request: jsonrpc_base::Request) -> Option<Self>;
}

impl<M> XcpRpcMethod for M
where
    M: TryToValue + TryFromValue + XcpRpcMethodNamed + Serialize + DeserializeOwned,
{
    fn write_xmlrpc<W: std::fmt::Write>(&self, w: &mut W) -> anyhow::Result<()> {
        w.write_str(r#"<?xml version="1.0"?>"#)?;

        let method = dxr::MethodCall::new(M::get_method_name().into(), vec![self.try_to_value()?]);
        quick_xml::se::to_writer(w, &method)?;

        Ok(())
    }

    fn try_from_xmlrpc(method: dxr::MethodCall) -> Option<Self> {
        if method.name() == M::get_method_name() {
            M::try_from_value(method.params().first()?).ok()
        } else {
            None
        }
    }

    fn write_jsonrpc<W: std::io::Write>(&self, w: &mut W) -> anyhow::Result<()> {
        jsonrpc_base::Request {
            jsonrpc: "2.0".into(),
            id: serde_json::Value::String(uuid::Uuid::new_v4().as_simple().to_string()),
            method: Self::get_method_name().into(),
            params: Some(serde_json::to_value(self)?),
        }
        .try_to_writer(w)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!(e.to_string()))
    }

    fn try_from_jsonrpc(request: jsonrpc_base::Request) -> Option<Self> {
        if request.method == M::get_method_name() {
            serde_json::from_value(request.params?).ok()
        } else {
            None
        }
    }
}

pub fn parse_method_xmlrpc(s: &str) -> anyhow::Result<dxr::MethodCall> {
    Ok(quick_xml::de::from_str(s)?)
}

/// s may be readed partially (chained requests)
pub fn parse_method_jsonrpc(s: &mut &[u8]) -> anyhow::Result<jsonrpc_base::Request> {
    Ok(serde_json::from_reader(s)?)
}
