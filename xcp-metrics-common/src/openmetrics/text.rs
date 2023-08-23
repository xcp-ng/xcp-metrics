use std::{
    borrow::Cow,
    fmt::Write,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::Result;

use crate::metrics::{
    Exemplar, Label, Metric, MetricFamily, MetricSet, MetricType, MetricValue, NumberValue,
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

    if !family.help.is_empty() {
        writeln!(writer, "# HELP {name} {}", escape_string(&family.help))?;
    }

    for metric in family.metrics.values() {
        write_metric(writer, &name, metric)?;
    }

    Ok(())
}

fn format_number_value(value: &NumberValue) -> String {
    match value {
        NumberValue::Double(value) => format!("{value:.4}"),
        NumberValue::Int64(value) => value.to_string(),
        NumberValue::Undefined => "0".to_string(),
    }
}

fn format_timestamp(system_time: &SystemTime) -> String {
    format!(
        "{}.0",
        system_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    )
}

fn format_label(name: &str, value: &str) -> String {
    format!(
        "{}=\"{}\"",
        format_name(name, false, true),
        value.escape_default().collect::<String>()
    )
}

fn format_labels(labels: &[Label]) -> String {
    labels
        .iter()
        .map(|label| format_label(&label.0, &label.1))
        .collect::<Vec<String>>() // TODO: Consider https://github.com/rust-lang/rust/issues/79524
        .join(",")
}

fn format_exemplar(exemplar: Option<&Exemplar>) -> String {
    match exemplar {
        Some(exemplar) => format!(
            " # {{{}}} {}",
            format_labels(&exemplar.labels),
            exemplar.value,
            // TODO: test exemplar timestamp
            // exemplar
            //     .timestamp
            //     .map(|ts| format_timestamp(&ts))
            //     .unwrap_or_default()
        ),
        None => String::new(),
    }
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
            } => {
                writeln!(
                    writer,
                    "{name}_total{{{}}} {}{}",
                    format_labels(&metric.labels),
                    format_number_value(total),
                    format_exemplar(exemplar.as_deref())
                )?;

                if let Some(ts) = created {
                    writeln!(
                        writer,
                        "{name}_created{{{}}} {}",
                        format_labels(&metric.labels),
                        format_timestamp(ts),
                    )?;
                }
            }
            MetricValue::Histogram {
                sum,
                count,
                created,
                buckets,
            } => {
                let formatted_label = format_labels(&metric.labels);

                for bucket in buckets.iter() {
                    writeln!(
                        writer,
                        "{name}_bucket{{le=\"{}\"}} {}{}",
                        bucket.upper_bound,
                        bucket.count,
                        format_exemplar(bucket.exemplar.as_deref())
                    )?;
                }

                writeln!(writer, "{name}_count{{{formatted_label}}} {count}")?;

                writeln!(
                    writer,
                    "{name}_sum{{{formatted_label}}} {}",
                    format_number_value(sum)
                )?;

                writeln!(
                    writer,
                    "{name}_created{{{formatted_label}}} {}",
                    format_timestamp(created)
                )?;
            }
            MetricValue::StateSet(states) => {
                let formatted_labels = format_labels(&metric.labels);

                for state in states.iter() {
                    writeln!(
                        writer,
                        "{name}{{{formatted_labels}{}{}}} {}",
                        if metric.labels.is_empty() { "" } else { "," },
                        format_label(name, &state.name),
                        Into::<u32>::into(state.enabled)
                    )?;
                }
            }
            MetricValue::Info(labels) => {
                writeln!(
                    writer,
                    "{name}_info{{{}{}{}}} 1",
                    format_labels(&metric.labels),
                    if metric.labels.is_empty() { "" } else { "," },
                    format_labels(labels)
                )?;
            }
            MetricValue::Summary {
                sum,
                count,
                created,
                quantile,
            } => {
                let formatted_label = format_labels(&metric.labels);

                for quantile in quantile.iter() {
                    writeln!(
                        writer,
                        "{name}_bucket{{quantile=\"{}\"}} {}",
                        quantile.quantile, quantile.value
                    )?;
                }

                writeln!(writer, "{name}_count{{{formatted_label}}} {count}")?;

                writeln!(
                    writer,
                    "{name}_sum{{{formatted_label}}} {}",
                    format_number_value(sum)
                )?;

                writeln!(
                    writer,
                    "{name}_created{{{formatted_label}}} {}",
                    format_timestamp(created)
                )?;
            }
        }
    }

    Ok(())
}
