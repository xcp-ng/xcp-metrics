use std::{rc::Rc, time::Duration};

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
                    metrics: xs
                        // Get the list of domains.
                        .directory(XBTransaction::Null, "/local/domains")
                        .unwrap_or_default()
                        .iter()
                        // Get and check if the target memory metric exists.
                        .filter_map(|domid| {
                            xs.read(XBTransaction::Null, domid.as_str())
                                .ok()
                                .and_then(|value| value.parse().ok())
                                .map(|memory_target: i64| (domid, memory_target))
                        })
                        .filter_map(|(domid, memory_target)| {
                            // Get the domain's vm UUID.
                            let vm_uuid = xs
                                .read(
                                    XBTransaction::Null,
                                    format!("/local/domains/{domid}/vm").as_str(),
                                )
                                .and_then(|vm_path| {
                                    xs.read(XBTransaction::Null, format!("{vm_path}/uuid").as_str())
                                })
                                .ok();

                            let mut labels = vec![Label("domain".into(), domid.clone().into())];

                            if let Some(vm_uuid) = vm_uuid {
                                labels.push(Label("owner".into(), format!("vm {vm_uuid}").into()));
                            }

                            // Make it a metric.
                            Some(SimpleMetric {
                                labels,
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
