use std::{collections::HashMap, rc::Rc, time::Duration};

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

fn make_memory_target_metric(xs: &Xs, domid: &str, memory_target: i64) -> SimpleMetric {
    let vm_uuid = get_domain_uuid(xs, domid);

    let mut labels = vec![Label("domain".into(), domid.into())];

    if let Some(vm_uuid) = vm_uuid {
        labels.push(Label("owner".into(), format!("vm {vm_uuid}").into()));
    }

    SimpleMetric {
        labels,
        value: MetricValue::Gauge(NumberValue::Int64(memory_target)),
    }
}

fn get_domain_uuid(xs: &Xs, domid: &str) -> Option<String> {
    xs.read(
        XBTransaction::Null,
        format!("/local/domain/{domid}/vm").as_str(),
    )
    .and_then(|vm_path| xs.read(XBTransaction::Null, format!("{vm_path}/uuid").as_str()))
    .ok()
}

fn get_memory_target_value(xs: &Xs, domid: &str) -> Option<i64> {
    xs.read(
        XBTransaction::Null,
        format!("/local/domain/{domid}/memory/target").as_str(),
    )
    .ok()
    .and_then(|value| {
        value
            .parse()
            .map_err(|err| {
                eprintln!("Memory target parse error {err:?}");
                err
            })
            .ok()
    })
}

fn generate_metrics(xs: &Xs) -> anyhow::Result<SimpleMetricSet> {
    let mut families: HashMap<String, SimpleMetricFamily> = HashMap::new();

    match xs.directory(XBTransaction::Null, "/vm") {
        Ok(vms) => {
            families.insert(
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
            );
        }
        Err(err) => println!("Unable to get vm list {err}"),
    }

    match xs.directory(XBTransaction::Null, "/local/domain") {
        Ok(domains) => {
            families.insert(
                "memory_target".into(),
                SimpleMetricFamily {
                    metric_type: MetricType::Gauge,
                    unit: "bytes".into(),
                    help: "Target of VM balloon driver".into(),
                    metrics: domains
                        .iter()
                        // Get target memory metric (if exists).
                        .filter_map(|domid| get_memory_target_value(xs, &domid).map(|m| (domid, m)))
                        // Make it a metric.
                        .map(|(domid, memory_target)| {
                            make_memory_target_metric(xs, &domid, memory_target)
                        })
                        .collect(),
                },
            );
        }
        Err(err) => println!("Unable to get domains list {err}"),
    }

    Ok(SimpleMetricSet { families })
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
