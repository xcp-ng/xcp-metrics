//! xcp-rrdd JSON data source parser and writer.
use std::{borrow::Cow, time::SystemTime};

use serde::{de::Error, Deserialize, Serialize};
use smol_str::{SmolStr, ToSmolStr};
use uuid::Uuid;

use crate::metrics::{Label, MetricValue, NumberValue};

/// Errors that can happen while parsing a data source.
#[derive(Copy, Clone, Debug)]
pub enum DataSourceParseError {
    InvalidPayload(&'static str),
}

impl std::fmt::Display for DataSourceParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for DataSourceParseError {}

/// Type of a data source.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DataSourceType {
    Gauge,
    Absolute,
    Derive,
}

/// Try to parse a data source type.
impl TryFrom<&str> for DataSourceType {
    type Error = DataSourceParseError;

    fn try_from(value: &str) -> Result<Self, DataSourceParseError> {
        match value.to_ascii_lowercase().as_str() {
            "gauge" => Ok(Self::Gauge),
            "absolute" => Ok(Self::Absolute),
            "derive" => Ok(Self::Derive),
            _ => Err(DataSourceParseError::InvalidPayload(
                "Unknown datasource type",
            )),
        }
    }
}

impl From<DataSourceType> for &'static str {
    fn from(val: DataSourceType) -> Self {
        match val {
            DataSourceType::Gauge => "gauge",
            DataSourceType::Absolute => "absolute",
            DataSourceType::Derive => "derive",
        }
    }
}

impl From<DataSourceType> for Cow<'static, str> {
    fn from(value: DataSourceType) -> Self {
        Cow::Borrowed(value.into())
    }
}

/// Owner of the data source.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum DataSourceOwner {
    Host,
    VM(Uuid),
    SR(Uuid),
}

/// Try to parse a data source owner.
/// UUID must be provided for VM and SR variants.
impl TryFrom<&str> for DataSourceOwner {
    type Error = DataSourceParseError;

    // TODO: Improve UUID parsing logic.
    // TODO: Cleanup this
    fn try_from(value: &str) -> Result<Self, DataSourceParseError> {
        let splitted: Vec<&str> = value.split_whitespace().collect();

        if let Some(kind) = splitted.first() {
            match kind.to_ascii_lowercase().as_str() {
                "host" => Ok(Self::Host),
                "vm" => Ok(Self::VM(
                    splitted.get(1).and_then(|u| u.parse().ok()).ok_or(
                        DataSourceParseError::InvalidPayload("Invalid owner VM UUID"),
                    )?,
                )),
                "sr" => Ok(Self::SR(
                    splitted.get(1).and_then(|u| u.parse().ok()).ok_or(
                        DataSourceParseError::InvalidPayload("Invalid owner SR UUID"),
                    )?,
                )),
                _ => Err(DataSourceParseError::InvalidPayload("Unknown owner kind")),
            }
        } else {
            Err(DataSourceParseError::InvalidPayload(
                "Unexpected owner value",
            ))
        }
    }
}

impl From<DataSourceOwner> for Box<str> {
    fn from(value: DataSourceOwner) -> Self {
        match value {
            DataSourceOwner::Host => "host".into(),
            DataSourceOwner::VM(uuid) => format!("vm {}", uuid.as_hyphenated()).into(),
            DataSourceOwner::SR(uuid) => format!("sr {}", uuid.as_hyphenated()).into(),
        }
    }
}

/// A data source value.
/// May be [DataSourceValue::Undefined] variant if missing or unexpected.
#[derive(Copy, Clone, PartialEq, Debug)]
pub enum DataSourceValue {
    Int64(i64),
    Float(f64),
    Undefined,
}

impl DataSourceValue {
    /// Parse a value and value_type into a [DataSourceValue].
    // TODO: Is v1 compatibility needed ?
    fn parse(
        value_type_str: &str,
        // v1 compatibility.
        value_str: Option<&str>,
    ) -> Result<Self, DataSourceParseError> {
        Ok(match value_type_str {
            "int64" => {
                if let Some(v) = value_str {
                    // Protocol v1 compatibility
                    DataSourceValue::Int64(v.parse().or(Err(
                        DataSourceParseError::InvalidPayload("Unable to parse 'value'"),
                    ))?)
                } else {
                    DataSourceValue::Int64(0)
                }
            }
            "float" => {
                if let Some(v) = value_str {
                    // Protocol v1 compatibility
                    DataSourceValue::Float(v.parse().or(Err(
                        DataSourceParseError::InvalidPayload("Unable to parse 'value'"),
                    ))?)
                } else {
                    DataSourceValue::Float(0.0)
                }
            }
            _ => DataSourceValue::Undefined,
        })
    }

    fn get_type_str(&self) -> Option<String> {
        match self {
            DataSourceValue::Int64(_) => Some("int64".into()),
            DataSourceValue::Float(_) => Some("float".into()),
            DataSourceValue::Undefined => None,
        }
    }
}

/// A non-parsed (strings) data source metadata structure.
/// Unusable unless converted to [DataSourceMetadata].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DataSourceMetadataRaw {
    pub description: Option<String>,
    pub units: Option<String>,
    #[serde(rename = "type")]
    pub ds_type: Option<String>,
    pub value: Option<String>,
    pub value_type: Option<String>,
    pub min: Option<String>,
    pub max: Option<String>,
    pub owner: Option<String>,
    pub default: Option<String>,
}

/// A metadata source.
#[derive(Clone, PartialEq, Debug)]
pub struct DataSourceMetadata {
    pub description: SmolStr,
    pub units: SmolStr,
    pub ds_type: DataSourceType,
    pub value: DataSourceValue,
    pub min: f32,
    pub max: f32,
    pub owner: DataSourceOwner,
    pub default: bool,
}

impl Serialize for DataSourceMetadata {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        DataSourceMetadataRaw::from(self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DataSourceMetadata {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match DataSourceMetadataRaw::deserialize(deserializer) {
            Ok(metadata_raw) => Ok((&metadata_raw).try_into().map_err(D::Error::custom)?),
            Err(e) => Err(e),
        }
    }
}

impl TryFrom<&DataSourceMetadataRaw> for DataSourceMetadata {
    type Error = DataSourceParseError;

    fn try_from(raw: &DataSourceMetadataRaw) -> Result<Self, Self::Error> {
        let description = raw.description.as_deref().unwrap_or_default().into();
        let units = raw.units.as_deref().unwrap_or_default().into();

        let ds_type = raw
            .ds_type
            .as_deref()
            .map_or_else(|| Ok(DataSourceType::Absolute), DataSourceType::try_from)?;

        let value = raw
            .value_type
            .as_deref()
            .map_or(Ok(DataSourceValue::Undefined), |value_type| {
                DataSourceValue::parse(value_type, raw.value.as_deref())
            })?;

        let min = raw.min.as_deref().map_or(Ok(f32::NEG_INFINITY), |s| {
            s.parse().or(Err(DataSourceParseError::InvalidPayload(
                "Unable to parse 'min'",
            )))
        })?;

        let max = raw.max.as_deref().map_or(Ok(f32::INFINITY), |s| {
            s.parse().or(Err(DataSourceParseError::InvalidPayload(
                "Unable to parse 'max'",
            )))
        })?;

        let owner = raw
            .owner
            .as_deref()
            .map_or(Ok(DataSourceOwner::Host), DataSourceOwner::try_from)?;

        let default = raw.default.as_deref().map_or(Ok(false), |s| {
            s.parse().or(Err(DataSourceParseError::InvalidPayload(
                "Unable to parse 'default",
            )))
        })?;

        Ok(Self {
            description,
            units,
            ds_type,
            value,
            min,
            max,
            owner,
            default,
        })
    }
}

impl From<&DataSourceMetadata> for DataSourceMetadataRaw {
    fn from(val: &DataSourceMetadata) -> Self {
        let description = if val.description.is_empty() {
            None
        } else {
            Some(val.description.to_string())
        };

        let units = Some(val.units.to_string());

        let ds_type = Some(<&str>::from(val.ds_type).to_string());

        let value = match val.value {
            DataSourceValue::Int64(v) => Some(v.to_string()),
            DataSourceValue::Float(v) => Some(v.to_string()),
            DataSourceValue::Undefined => None,
        };
        let value_type = val.value.get_type_str();

        let default = Some(val.default.to_string());

        let min = Some(val.min.to_string());
        let max = Some(val.max.to_string());

        let owner = Some(<Box<str>>::from(val.owner).into());

        Self {
            description,
            default,
            ds_type,
            units,
            value,
            value_type,
            min,
            max,
            owner,
        }
    }
}

impl Default for DataSourceMetadata {
    fn default() -> Self {
        Self {
            description: Default::default(),
            units: Default::default(),
            ds_type: DataSourceType::Absolute,
            value: DataSourceValue::Undefined,
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
            owner: DataSourceOwner::Host,
            default: false,
        }
    }
}

impl crate::metrics::Metric {
    pub fn from_protocol_v2(
        metadata: &DataSourceMetadata,
        value: DataSourceValue,
        // Used for derive creation timestamp.
        created: Option<SystemTime>,
    ) -> Self {
        Self {
            labels: vec![Label {
                name: "owner".into(),
                value: <Box<str>>::from(metadata.owner).to_smolstr(),
            }]
            .into_boxed_slice(),
            value: MetricValue::from_protocol_v2(metadata, value, created),
        }
    }
}

impl crate::metrics::MetricValue {
    pub fn from_protocol_v2(
        metadata: &DataSourceMetadata,
        value: DataSourceValue,
        // Used for derive creation timestamp.
        created: Option<SystemTime>,
    ) -> Self {
        match metadata.ds_type {
            DataSourceType::Gauge => MetricValue::Gauge(value.into()),
            DataSourceType::Derive | DataSourceType::Absolute => MetricValue::Counter {
                total: value.into(),
                created,
                exemplar: None,
            },
        }
    }
}

impl From<DataSourceValue> for NumberValue {
    fn from(value: DataSourceValue) -> Self {
        match value {
            DataSourceValue::Int64(val) => Self::Int64(val),
            DataSourceValue::Float(val) => Self::Double(val),
            DataSourceValue::Undefined => Self::Undefined,
        }
    }
}

impl From<NumberValue> for DataSourceValue {
    fn from(value: NumberValue) -> Self {
        match value {
            NumberValue::Double(val) => Self::Float(val),
            NumberValue::Int64(val) => Self::Int64(val),
            NumberValue::Undefined => Self::Undefined,
        }
    }
}
