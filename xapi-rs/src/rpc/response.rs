//! RPC response types.
use dxr::{TryFromValue, TryToValue};
use serde::{Deserialize, Serialize};

/// Response to `Plugin.Metrics.get_versions`
#[derive(Default, Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
pub struct PluginMetricsVersionsResponse {
    pub versions: Vec<String>,
}
