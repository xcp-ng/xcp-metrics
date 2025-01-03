//! RRD Xport format and serialization.
use std::{
    fmt::Write,
    time::{SystemTime, SystemTimeError},
};

use serde::Serialize;

use crate::utils::write_bridge::WriterWrapper;

#[derive(Debug, Clone)]
pub struct RrdXport {
    pub start: SystemTime,
    pub end: SystemTime,
    pub step_secs: u32,

    pub legend: Vec<Box<str>>,
    pub data: Vec<(SystemTime, Box<[f64]>)>,
}

impl RrdXport {
    fn write_metadata_xml<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        let writer = &mut WriterWrapper(writer);

        write!(writer, "<meta>")?;

        write!(
            writer,
            "<start>{}</start>",
            self.start.duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
        )?;

        write!(writer, "<step>{}</step>", self.step_secs)?;

        write!(
            writer,
            "<end>{}</end>",
            self.end.duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
        )?;

        write!(writer, "<rows>{}</rows>", self.data.len())?;
        write!(writer, "<columns>{}</columns>", self.legend.len())?;

        write!(writer, "<legend>")?;
        for entry in &self.legend {
            write!(writer, "<entry>{entry}</entry>")?;
        }
        write!(writer, "</legend>")?;

        write!(writer, "</meta>")?;

        Ok(())
    }

    fn write_data_xml<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        let writer = &mut WriterWrapper(writer);

        write!(writer, "<data>")?;

        for (t, values) in &self.data {
            write!(
                writer,
                "<row><t>{}</t>",
                t.duration_since(SystemTime::UNIX_EPOCH)?.as_secs()
            )?;

            for value in values.iter() {
                write!(writer, "<v>{value}</v>")?;
            }

            write!(writer, "</row>")?;
        }

        write!(writer, "</data>")?;
        Ok(())
    }

    pub fn write_xml<W: std::io::Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        let writer = &mut WriterWrapper(writer);

        write!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;
        write!(writer, "<xport>")?;

        self.write_metadata_xml(writer.0)?;
        self.write_data_xml(writer.0)?;
        write!(writer, "<script />")?;

        write!(writer, "</xport>")?;

        Ok(())
    }
}

// JSON support

#[derive(Serialize)]
struct RrdXportJsonMeta<'a> {
    start: u64,
    step: u32,
    end: u64,
    rows: u32,
    columns: u32,
    legend: &'a [Box<str>],
}

#[derive(Serialize)]
struct RrdXportJsonData<'a> {
    timestamp: u64,
    values: &'a [f64],
}

#[derive(Serialize)]
struct RrdXportJson<'a> {
    meta: RrdXportJsonMeta<'a>,
    data: Vec<RrdXportJsonData<'a>>,
}

fn to_epoch(timestamp: SystemTime) -> Result<u64, SystemTimeError> {
    Ok(timestamp.duration_since(SystemTime::UNIX_EPOCH)?.as_secs())
}

impl<'a> TryFrom<&'a RrdXport> for RrdXportJson<'a> {
    type Error = SystemTimeError;

    fn try_from(rrd: &'a RrdXport) -> Result<Self, Self::Error> {
        Ok(Self {
            meta: RrdXportJsonMeta {
                start: to_epoch(rrd.start)?,
                step: rrd.step_secs,
                end: to_epoch(rrd.end)?,
                rows: rrd.data.len() as _,
                columns: rrd.legend.len() as _,
                legend: &rrd.legend,
            },
            data: rrd
                .data
                .iter()
                .map(|(timestamp, content)| {
                    Ok(RrdXportJsonData {
                        timestamp: to_epoch(*timestamp)?,
                        values: content.as_ref(),
                    })
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

impl RrdXport {
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(&RrdXportJson::try_from(self)?)?)
    }

    pub fn to_json5(&self) -> anyhow::Result<String> {
        Ok(json5::to_string(&RrdXportJson::try_from(self)?)?)
    }
}
