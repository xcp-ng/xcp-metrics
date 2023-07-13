use anyhow::Result;
use std::{
    borrow::Cow,
    fmt::Write,
    time::{SystemTime, UNIX_EPOCH},
};

use xcp_metrics_common::metrics::{
    Label, Metric, MetricFamily, MetricSet, MetricType, MetricValue, NumberValue,
};

fn metric_type_to_str(metric_type: MetricType) -> &'static str {
    match metric_type {
        MetricType::Unknown => "unknown",
        MetricType::Gauge => "gauge",
        MetricType::Counter => "counter",
        MetricType::StateSet => "stateset",
        MetricType::Info => "info",
        MetricType::Histogram => "histogram",
        MetricType::GaugeHistogram => "gaugehistogram",
        MetricType::Summary => "summary",
    }
}

fn escape_string(s: &str) -> String {
    s.escape_default().collect()
}

fn format_name(s: &str, allow_colon: bool, keep_underscores: bool) -> String {
    s.char_indices()
        .filter_map(|(pos, c)| match c {
            c @ 'A'..='Z' => Some(c),
            c @ 'a'..='z' => Some(c),
            c @ '0'..='9' if pos != 0 => Some(c),
            ':' if allow_colon => Some(':'),
            '_' if keep_underscores => Some('_'),
            _ => None,
        })
        .collect()
}

pub fn write_metrics_set_text<W: Write>(writer: &mut W, metrics: &MetricSet) -> Result<()> {
    for (name, family) in &metrics.families {
        let name = format_name(name, true, true);

        write_family(writer, &name, family)?;
    }

    writeln!(writer, "# EOF")?;

    Ok(())
}

fn write_family<W: Write>(writer: &mut W, name: &str, family: &MetricFamily) -> Result<()> {
    // Remove all non-ascii characters from unit.
    let unit_escaped = format_name(&family.unit, true, false);

    // Add the unit suffix (if relevant)
    let name = if !unit_escaped.is_empty() {
        Cow::Owned(format!("{name}_{unit_escaped}"))
    } else {
        Cow::Borrowed(name)
    };

    writeln!(
        writer,
        "# TYPE {name} {}",
        metric_type_to_str(family.metric_type)
    )?;

    if !name.is_empty() {
        writeln!(writer, "# UNIT {name} {}", unit_escaped)?;
    }

    writeln!(writer, "# HELP {name} {}", escape_string(&family.help))?;

    for metric in family.metrics.values() {
        write_metric(writer, &name, metric)?;
    }

    Ok(())
}

fn format_number_value(value: &NumberValue) -> String {
    match value {
        NumberValue::Double(value) => value.to_string(),
        NumberValue::Int64(value) => value.to_string(),
        NumberValue::Undefined => "0".to_string(),
    }
}

fn format_timestamp(system_time: &SystemTime) -> String {
    system_time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

fn format_labels(labels: &[Label]) -> String {
    labels
        .iter()
        .map(|label| {
            format!(
                "{}=\"{}\"",
                format_name(&label.0, false, true),
                label.1.escape_default().collect::<String>()
            )
        })
        .collect()
}

fn write_metric<W: Write>(writer: &mut W, name: &str, metric: &Metric) -> Result<()> {
    for metric_point in metric.metrics_point.iter() {
        match &metric_point.value {
            MetricValue::Unknown(value) | MetricValue::Gauge(value) => {
                writeln!(
                    writer,
                    "{name}{{{}}} {}",
                    format_labels(&metric.labels),
                    format_number_value(value),
                    // FIXME: timestamp breaks prometheus for some reason
                    // format_timestamp(&metric_point.timestamp)
                )?;
            }
            MetricValue::Counter {
                total,
                created,
                exemplar,
            } => todo!(),
            MetricValue::Histogram {
                sum,
                count,
                created,
                buckets,
            } => todo!(),
            MetricValue::StateSet(_) => todo!(),
            MetricValue::Info(_) => todo!(),
            MetricValue::Summary {
                sum,
                count,
                created,
                quantile,
            } => todo!(),
        }
    }

    Ok(())
}
