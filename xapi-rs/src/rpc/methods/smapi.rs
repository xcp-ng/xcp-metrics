//! RPC methods structures.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::rpc_method;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginQuery {
    /// Debug context from the caller
    pub dbg: String,
}

rpc_method!(PluginQuery, "Plugin.query");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginSrList {
    pub dbg: String,
}

rpc_method!(PluginSrList, "Plugin.query");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginDiagnostics {
    pub dbg: String,
}

rpc_method!(PluginDiagnostics, "Plugin.diagnostics");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VolumeHealthStatus {
    Healthy,
    Recovering,
    Unreachable,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VolumeSrStat {
    pub sr: String,
    pub name: String,
    pub uuid: Option<uuid::Uuid>,
    pub description: String,
    pub free_space: i64,
    pub total_space: i64,
    pub datasources: Vec<String>,
    pub clustered: bool,
    pub health: (VolumeHealthStatus, String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VolumeProbeResult {
    pub configuration: HashMap<String, String>,
    pub complete: bool,
    pub sr: VolumeSrStat,
    pub extra_info: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VolumeType {
    Data,
    #[serde(rename = "CBT_Metadata")]
    CbtMetadata,
    #[serde(rename = "Data_and_CBT_Metadata")]
    DataAndCbtMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Volume {
    pub key: String,
    pub uuid: Option<uuid::Uuid>,
    pub name: String,
    pub description: String,
    pub read_write: bool,
    pub sharable: bool,
    pub virtual_size: i64,
    pub physical_utilisation: i64,
    pub uri: Vec<String>,
    pub keys: HashMap<String, String>,
    pub volume_type: Option<VolumeType>,
    pub cbt_enabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VolumeBlockList {
    pub blocksize: i64,
    pub ranges: Vec<(i64, i64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VolumeChangedBlocks {
    pub granularity: i64,
    /// as base64
    pub bitmap: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VolumeProbe {}

rpc_method!(VolumeProbe, "SR.probe");
