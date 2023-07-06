use dxr::{TryFromValue, TryToValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PluginLocalRegisterResponse {
    #[serde(rename = "$value")]
    pub next_reading: f64
}
