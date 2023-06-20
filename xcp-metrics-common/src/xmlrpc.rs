use dxr::{MethodCall, TryToValue};
use std::io::Write;

macro_rules! rpc_method {
    ($struct:ty, $name:stmt) => {
        impl XcpRpcMethodNamed for $struct {
            fn get_method_name() -> &'static str {
                $name
            }
        }
    };
}

trait XcpRpcMethodNamed {
    fn get_method_name() -> &'static str;
}

pub trait XcpRpcMethod {
    fn write_xmlrpc<W: Write>(&self, w: &mut W) -> anyhow::Result<()>;
}

impl<M> XcpRpcMethod for M
where
    M: TryToValue + XcpRpcMethodNamed,
{
    fn write_xmlrpc<W: Write>(&self, w: &mut W) -> anyhow::Result<()> {
        w.write_all(r#"<?xml version="1.0"?>"#.as_bytes())?;

        let method = MethodCall::new(M::get_method_name().into(), vec![self.try_to_value()?]);
        quick_xml::se::to_writer(w, &method)?;

        Ok(())
    }
}

#[derive(Clone, TryToValue)]
pub struct PluginLocalRegister {
    pub protocol: String,
    pub info: String,
    pub uid: String,
}

rpc_method!(PluginLocalRegister, "Plugin.Local.register");
