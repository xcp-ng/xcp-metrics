use std::{iter, rc::Rc, time::Duration};

use tokio::time;
use xcp_metrics_common::metrics::{Label, MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::protocol_v3::{
    utils::{SimpleMetric, SimpleMetricFamily, SimpleMetricSet},
    MetricsPlugin,
};
use xenstore_rs::{XBTransaction, Xs, XsOpenFlags};

pub fn get_vm_infos(xs: &Xs, vm_uuid: &str, attributes: &[&str]) -> MetricValue {
    MetricValue::Info(
        attributes
            .iter()
            .filter_map(|&attr| {
                xs.read(
                    XBTransaction::Null,
                    format!("/vm/{vm_uuid}/{attr}").as_str(),
                )
                .ok()
                .map(|value| Label(attr.into(), value.into()))
            })
            .collect(),
    )
}

pub fn get_domain_paths(xs: &Xs, vm_uuid: &str) -> Vec<String> {
    xs.directory(
        XBTransaction::Null,
        format!("/vm/{vm_uuid}/domains").as_str(),
    )
    .unwrap_or_default()
    .iter()
    .filter_map(|domain_path| xs.read(XBTransaction::Null, &domain_path).ok())
    .collect()
}

fn generate_metrics(xs: &Xs) -> anyhow::Result<SimpleMetricSet> {
    let vms = xs.directory(XBTransaction::Null, "/vm")?;

    Ok(SimpleMetricSet {
        families: [
            (
                "vm_info".into(),
                SimpleMetricFamily {
                    metric_type: MetricType::Info,
                    unit: "".into(),
                    help: "Virtual machine informations".into(),
                    metrics: vms
                        .iter()
                        // Get vm metrics.
                        .map(|uuid| SimpleMetric {
                            labels: vec![Label("owner".into(), format!("vm {uuid}").into())].into(),
                            value: get_vm_infos(&xs, &uuid, &["name"]),
                        })
                        .collect(),
                },
            ),
            (
                "memory_target".into(),
                SimpleMetricFamily {
                    metric_type: MetricType::Gauge,
                    unit: "bytes".into(),
                    help: "Target of VM balloon driver".into(),
                    metrics: vms
                        .iter()
                        // Combine uuid and domain paths.
                        .flat_map(|uuid: &String| {
                            iter::zip(
                                iter::repeat(uuid.as_str()),
                                get_domain_paths(xs, uuid.as_str()),
                            )
                        })
                        // Generate the target metric (if exists), using domain_path/memory/target
                        .filter_map(|(vm_uuid, domain_path)| {
                            xs.read(
                                XBTransaction::Null,
                                format!("{domain_path}/memory/target").as_str(),
                            )
                            .ok()
                            // Try to parse memory-target amount.
                            .and_then(|v| v.parse::<i64>().ok())
                            .map(|memory_target| SimpleMetric {
                                labels: vec![Label("owner".into(), format!("vm {vm_uuid}").into())],
                                value: MetricValue::Gauge(NumberValue::Int64(memory_target)),
                            })
                        })
                        .collect(),
                },
            ),
        ]
        .into_iter()
        .collect(),
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let xs = Rc::new(Xs::new(XsOpenFlags::ReadOnly).map_err(|e| anyhow::anyhow!("{e}"))?);

    let plugin = MetricsPlugin::new(
        "xcp-metrics-plugin-xenstored",
        generate_metrics(&xs)?.into(),
    )
    .await?;

    loop {
        // Fetch and push new metrics.
        plugin.update(generate_metrics(&xs)?.into()).await.unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }
}
