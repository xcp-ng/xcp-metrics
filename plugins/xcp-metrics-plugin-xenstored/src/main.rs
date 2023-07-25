use std::{rc::Rc, time::Duration};

use tokio::time;
use xcp_metrics_common::metrics::{Label, MetricType, MetricValue};
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
    Ok(SimpleMetricSet {
        families: [(
            "vm_info".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Info,
                unit: "".into(),
                help: "Virtual machine informations".into(),
                metrics: xs
                    // Read all vm handles.
                    .directory(XBTransaction::Null, "/vm")?
                    .into_iter()
                    // Get vm metrics.
                    .map(|uuid| SimpleMetric {
                        labels: vec![Label("owner".into(), format!("vm {uuid}").into())].into(),
                        value: get_vm_infos(&xs, &uuid, &["name"]),
                    })
                    .collect(),
            },
        )]
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
