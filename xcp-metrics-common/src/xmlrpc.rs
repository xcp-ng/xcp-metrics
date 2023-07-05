use std::fmt::Write;

use dxr::{MethodCall, TryFromValue, TryToValue};

pub use dxr;

macro_rules! rpc_method {
    ($struct:ty, $name:stmt) => {
        impl XcpRpcMethodNamed for $struct {
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
    fn write_xmlrpc<W: Write>(&self, w: &mut W) -> anyhow::Result<()>;
    fn try_from_method(call: MethodCall) -> Option<Self>;
}

impl<M> XcpRpcMethod for M
where
    M: TryToValue + TryFromValue + XcpRpcMethodNamed,
{
    fn write_xmlrpc<W: Write>(&self, w: &mut W) -> anyhow::Result<()> {
        w.write_str(r#"<?xml version="1.0"?>"#)?;

        let method = MethodCall::new(M::get_method_name().into(), vec![self.try_to_value()?]);
        quick_xml::se::to_writer(w, &method)?;

        Ok(())
    }

    fn try_from_method(method: MethodCall) -> Option<Self> {
        if method.name() == M::get_method_name() {
            M::try_from_value(method.params().first()?).ok()
        } else {
            None
        }
    }
}

pub fn parse_method(s: &str) -> anyhow::Result<MethodCall> {
    Ok(quick_xml::de::from_str(s)?)
}

#[derive(Debug, Clone, TryToValue, TryFromValue)]
pub struct PluginLocalRegister {
    pub protocol: String,
    pub info: String,
    pub uid: String,
}

rpc_method!(PluginLocalRegister, "Plugin.Local.register");

#[derive(Debug, Clone, TryToValue, TryFromValue)]
pub struct PluginLocalDeregister {
    pub uid: String,
}

rpc_method!(PluginLocalDeregister, "Plugin.Local.deregister");

#[derive(Debug, Clone, TryToValue, TryFromValue)]
pub struct PluginLocalNextReading {}

rpc_method!(PluginLocalNextReading, "Plugin.Local.next_reading");
