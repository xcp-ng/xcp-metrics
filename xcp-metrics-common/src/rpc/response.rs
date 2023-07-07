use dxr::{TryFromValue, TryToValue};
use serde::{Deserialize, Serialize};

/// The response for a `Plugin.Local.register` request.
#[derive(Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginLocalRegisterResponse {
    /// The time before the next plugin reading.
    #[serde(rename = "$value")]
    pub next_reading: f64
}
