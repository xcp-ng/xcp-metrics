//! XML serialization utilities.
use std::fmt::Write;

use quick_xml::se::Serializer;
use serde::Serialize;

pub(super) fn xml_to_string<V: Serialize>(value: &V) -> anyhow::Result<String> {
    let mut buffer = String::new();

    buffer.write_str(r#"<?xml version="1.0"?>"#)?;

    let mut serializer = Serializer::new(&mut buffer);
    serializer.expand_empty_elements(true);

    value.serialize(serializer)?;

    Ok(buffer)
}
