use std::io::Write;

use quick_xml::se::Serializer;
use serde::Serialize;

use crate::utils::write_bridge::WriterWrapper;

pub(super) fn write_xml<V: Serialize, W: Write>(w: &mut W, value: &V) -> anyhow::Result<()> {
    w.write_all(r#"<?xml version="1.0"?>"#.as_bytes())?;

    let mut writer = WriterWrapper(w);

    let mut serializer = Serializer::new(&mut writer);
    serializer.expand_empty_elements(true);

    Ok(value.serialize(serializer)?)
}

pub(super) fn xml_to_string<V: Serialize>(value: &V) -> anyhow::Result<String> {
    let mut buffer = vec![];
    write_xml(&mut buffer, value)?;
    Ok(String::from_utf8(buffer)?)
}
