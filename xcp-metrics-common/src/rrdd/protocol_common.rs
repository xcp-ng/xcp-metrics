//! xcp-rrdd JSON data source parser and writer.
use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use serde_json;

/// Errors that can happen while parsing a data source.
#[derive(Debug)]
pub enum DataSourceParseError {
    InvalidPayload(&'static str),
}

/// Type of a data source.
#[derive(Clone, PartialEq, Eq, Debug)]
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
#[derive(Clone, PartialEq, Eq, Debug)]
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

        if let Some(kind) = splitted.get(0) {
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
            return Err(DataSourceParseError::InvalidPayload(
                "Unexpected owner value",
            ));
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
#[derive(Clone, PartialEq, Debug)]
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
        // v1 compatibility, Option<String> to prevent dangling &str compilation error.
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
    pub description: Box<str>,
    pub units: Box<str>,
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
        Into::<DataSourceMetadataRaw>::into(self).serialize(serializer)
    }
}

impl TryFrom<&DataSourceMetadataRaw> for DataSourceMetadata {
    type Error = DataSourceParseError;

    fn try_from(raw: &DataSourceMetadataRaw) -> Result<Self, Self::Error> {
        let description = raw.description.as_deref().unwrap_or_default().into();
        let units = raw.units.as_deref().unwrap_or_default().into();

        let ds_type = raw.ds_type.as_deref().map_or_else(
            || Ok(DataSourceType::Absolute),
            |s| DataSourceType::try_from(s),
        )?;

        let value = raw
            .value_type
            .as_deref()
            .map_or(Ok(DataSourceValue::Undefined), |value_type| {
                DataSourceValue::parse(&value_type, raw.value.as_deref())
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
            .map_or(Ok(DataSourceOwner::Host), |s| DataSourceOwner::try_from(s))?;

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
            Some(val.description.as_ref().into())
        };

        let units = Some(val.units.as_ref().into());

        let ds_type = Some(Into::<&str>::into(val.ds_type.clone()).to_string());

        let value = match val.value {
            DataSourceValue::Int64(v) => Some(v.to_string()),
            DataSourceValue::Float(v) => Some(v.to_string()),
            DataSourceValue::Undefined => None,
        };
        let value_type = val.value.get_type_str();

        let default = Some(val.default.to_string());

        let min = Some(val.min.to_string());
        let max = Some(val.max.to_string());

        let owner = Some(Into::<Box<str>>::into(val.owner.clone()).into());

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
