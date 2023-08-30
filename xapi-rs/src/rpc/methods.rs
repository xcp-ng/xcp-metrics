use dxr::{TryFromValue, TryToValue};
use serde::{Deserialize, Serialize};

use crate::rpc_method;

/// `Plugin.Local.register` registers a plugin as a source of a set of data-sources. `uid` is a unique identifier
/// for the plugin, often the name of the plugin. `info` is the RRD frequency, and `protocol` specifies whether
/// the plugin will be using V1 or V2 of the protocol.
#[derive(Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
pub struct PluginLocalRegister {
    pub protocol: String,
    pub info: String,
    pub uid: String,
}

rpc_method!(PluginLocalRegister, "Plugin.Local.register");

/// Deregisters a plugin by uid.
#[derive(Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
pub struct PluginLocalDeregister {
    pub uid: String,
}

rpc_method!(PluginLocalDeregister, "Plugin.Local.deregister");

/// Returns the number of seconds until the next reading will be taken.
#[derive(Default, Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
pub struct PluginLocalNextReading {}

rpc_method!(PluginLocalNextReading, "Plugin.Local.next_reading");

/// Fetch the metrics in the OpenMetrics format.
#[derive(Default, Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
pub struct OpenMetricsMethod {
    pub protobuf: bool,
}

rpc_method!(OpenMetricsMethod, "OpenMetrics");

/// Get the list of OpenMetrics versions that are supported.
#[derive(Default, Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
pub struct PluginMetricsGetVersions {}

rpc_method!(PluginMetricsGetVersions, "Plugin.Metrics.get_versions");

/// `Plugin.Metrics.register` registers a plugin as a source of metrics using protocol V3. `uid` is a unique identifier
/// for the plugin, often the name of the plugin. `protocol` specifies the OpenMetrics version to use.
#[derive(Default, Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
pub struct PluginMetricsRegister {
    pub uid: String,
    pub protocol: String,
}

rpc_method!(PluginMetricsRegister, "Plugin.Metrics.register");
