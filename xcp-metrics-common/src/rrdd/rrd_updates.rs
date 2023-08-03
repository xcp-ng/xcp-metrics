use std::{fmt::Write, time::SystemTime};

#[derive(Debug, Clone)]
pub struct RrdXport {
    pub start: SystemTime,
    pub end: SystemTime,
    pub step_secs: u32,

    pub legend: Vec<Box<str>>,
    pub data: Vec<(SystemTime, Box<[f64]>)>,
}

impl RrdXport {
    fn write_metadata_xml<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
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

    fn write_data_xml<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
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

    pub fn write_xml<W: Write>(&self, writer: &mut W) -> anyhow::Result<()> {
        write!(writer, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>")?;

        write!(writer, "<xport>")?;
        self.write_metadata_xml(writer)?;
        self.write_data_xml(writer)?;

        write!(writer, "<script />")?;
        write!(writer, "</xport>")?;

        Ok(())
    }
}
