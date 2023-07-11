use anyhow::Result;
use std::{
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

fn escaped_string(s: &str) -> String {
    s.escape_default().collect()
}

pub fn write_metrics_set_text<W: Write>(writer: &mut W, metrics: &MetricSet) -> Result<()> {
    for (name, family) in &metrics.families {
        let name = name.replace('-', "_");

        write_family(writer, &name, &family)?;
    }

    writeln!(writer, "# EOF")?;

    Ok(())
}

fn write_family<W: Write>(writer: &mut W, name: &str, family: &MetricFamily) -> Result<()> {
    writeln!(
        writer,
        "# TYPE {name} {}",
        metric_type_to_str(family.metric_type)
    )?;

    //let unit = family.unit.replace(&['(', ')', '\n'], "").to_lowercase();
    //writeln!(writer, "# UNIT {name} {}", unit)?;

    writeln!(writer, "# HELP {name} {}", escaped_string(&family.help))?;

    //let new_name = format!("{name}_{unit}");

    for (_, metric) in &family.metrics {
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
        .as_secs_f64()
        .to_string()
}

fn format_labels(labels: &[Label]) -> String {
    let formatted_labels: String = labels
        .iter()
        .map(|label| {
            format!(
                "{}=\"{}\"",
                label.0,
                label.1.escape_default().collect::<String>()
            )
        })
        .collect();

    format!("{{{}}}", formatted_labels)
}

fn write_metric<W: Write>(writer: &mut W, name: &str, metric: &Metric) -> Result<()> {
    for metric_point in metric.metrics_point.iter() {
        match &metric_point.value {
            MetricValue::Unknown(value) | MetricValue::Gauge(value) => {
                writeln!(
                    writer,
                    "{name}{} {} {}",
                    format_labels(&metric.labels),
                    format_number_value(value),
                    format_timestamp(&metric_point.timestamp)
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
