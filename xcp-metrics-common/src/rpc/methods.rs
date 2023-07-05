use dxr::{TryFromValue, TryToValue};
use serde::{Deserialize, Serialize};

use crate::rpc_method;

#[derive(Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize)]
pub struct PluginLocalRegister {
    pub protocol: String,
    pub info: String,
    pub uid: String,
}

rpc_method!(PluginLocalRegister, "Plugin.Local.register");

#[derive(Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize)]
pub struct PluginLocalDeregister {
    pub uid: String,
}

rpc_method!(PluginLocalDeregister, "Plugin.Local.deregister");

#[derive(Default, Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize)]
pub struct PluginLocalNextReading {}

rpc_method!(PluginLocalNextReading, "Plugin.Local.next_reading");

#[derive(Default, Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize)]
pub struct OpenMetricsMethod {}

rpc_method!(OpenMetricsMethod, "OpenMetrics");
