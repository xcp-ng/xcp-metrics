mod plugin;
mod watch_cache;

use std::{collections::HashMap, time::Duration};

use plugin::PluginState;
use tokio::time;
use xcp_metrics_common::metrics::{Label, MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::protocol_v3::{
    utils::{SimpleMetric, SimpleMetricFamily, SimpleMetricSet},
    MetricsPlugin,
};
use xenstore_rs::{Xs, XsOpenFlags};

pub fn get_vm_infos(plugin: &PluginState, vm_uuid: &str, attributes: &[&str]) -> MetricValue {
    MetricValue::Info(
        attributes
            .iter()
            .filter_map(|&attr| {
                plugin
                    .read(format!("/vm/{vm_uuid}/{attr}").as_str())
                    .map(|value| Label(attr.into(), value.into()))
            })
            .collect(),
    )
}

fn make_memory_target_metric(
    plugin: &PluginState,
    domid: &str,
    memory_target: i64,
) -> SimpleMetric {
    let vm_uuid = get_domain_uuid(plugin, domid);

    let mut labels = vec![Label("domain".into(), domid.into())];

    if let Some(vm_uuid) = vm_uuid {
        labels.push(Label("owner".into(), format!("vm {vm_uuid}").into()));
    }

    SimpleMetric {
        labels,
        value: MetricValue::Gauge(NumberValue::Int64(memory_target)),
    }
}

fn get_domain_uuid(plugin: &PluginState, domid: &str) -> Option<String> {
    plugin
        .read(format!("/local/domain/{domid}/vm").as_str())
        .and_then(|vm_path| plugin.read(format!("{vm_path}/uuid").as_str()))
}

fn get_memory_target_value(plugin: &PluginState, domid: &str) -> Option<i64> {
    plugin
        .read(format!("/local/domain/{domid}/memory/target").as_str())
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

fn generate_metrics(plugin: &mut PluginState, xs: &Xs) -> anyhow::Result<SimpleMetricSet> {
    if let Err(e) = plugin.update_domains(xs) {
        eprintln!("Unable to get domains: {e}");
    }

    if let Err(e) = plugin.update_vms(xs) {
        eprintln!("Unable to get vms: {e}");
    }

    let mut families: HashMap<String, SimpleMetricFamily> = HashMap::new();

    families.insert(
        "vm_info".into(),
        SimpleMetricFamily {
            metric_type: MetricType::Info,
            unit: "".into(),
            help: "Virtual machine informations".into(),
            metrics: plugin
                .vms
                .iter()
                // Get vm metrics.
                .map(|uuid| SimpleMetric {
                    labels: vec![Label("owner".into(), format!("vm {uuid}").into())],
                    value: get_vm_infos(plugin, uuid, &["name"]),
                })
                .collect(),
        },
    );

    families.insert(
        "memory_target".into(),
        SimpleMetricFamily {
            metric_type: MetricType::Gauge,
            unit: "bytes".into(),
            help: "Target of VM balloon driver".into(),
            metrics: plugin
                .domains
                .iter()
                // Get target memory metric (if exists).
                .filter_map(|domid| get_memory_target_value(plugin, domid).map(|m| (domid, m)))
                // Make it a metric.
                .map(|(domid, memory_target)| {
                    make_memory_target_metric(plugin, domid, memory_target)
                })
                .collect(),
        },
    );

    Ok(SimpleMetricSet { families })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let xs = Xs::new(XsOpenFlags::ReadOnly).map_err(|e| anyhow::anyhow!("{e}"))?;

    let mut plugin_state = PluginState::default();

    let plugin = MetricsPlugin::new(
        "xcp-metrics-plugin-xenstored",
        generate_metrics(&mut plugin_state, &xs)?.into(),
        None,
    )
    .await?;

    loop {
        // Fetch and push new metrics.
        plugin
            .update(generate_metrics(&mut plugin_state, &xs)?.into())
            .await
            .unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }
}
