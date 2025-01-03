//! RPC methods structures.
use std::collections::HashMap;

use dxr::{TryFromValue, TryToValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, TryToValue, TryFromValue, Serialize, Deserialize, PartialEq)]
pub struct QueryResult {
    pub plugin: String,
    pub name: String,
    pub description: String,
    pub vendor: String,
    pub copyright: String,
    pub version: String,
    pub required_api_version: String,
    pub features: Vec<String>,
    pub configuration: HashMap<String, String>,
    pub required_cluster_stack: Vec<String>,
}
