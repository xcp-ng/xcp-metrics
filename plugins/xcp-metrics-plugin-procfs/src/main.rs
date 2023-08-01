use std::{collections::HashMap, fs, time::Duration};
use tokio::time;

use xcp_metrics_common::metrics::{Label, MetricType, MetricValue};
use xcp_metrics_plugin_common::protocol_v3::{
    utils::{SimpleMetric, SimpleMetricFamily, SimpleMetricSet},
    MetricsPlugin,
};

fn generate_metrics() -> SimpleMetricSet {
    let pids = fs::read_dir("/proc")
        .expect("Unable to read /proc")
        .filter_map(|entry| entry.ok())
        .filter_map(|dir| {
            // Filter pids
            let file_name = dir.file_name();
            let raw_pid = file_name.to_string_lossy();

            raw_pid
                .chars()
                .all(|c| c.is_ascii_digit())
                .then(|| (raw_pid.to_string(), dir.path().to_path_buf()))
        })
        .collect::<Vec<_>>();

    let mut families: HashMap<String, SimpleMetricFamily> = HashMap::new();

    families.insert(
        "process_path".into(),
        SimpleMetricFamily {
            metric_type: MetricType::Info,
            unit: "".into(),
            help: "Process informations".into(),
            metrics: pids
                .iter()
                .map(|(pid, path)| SimpleMetric {
                    labels: vec![Label("pid".into(), pid.to_owned().into())],
                    value: MetricValue::Info(
                        vec![Label(
                            "cmdline".into(),
                            fs::read_to_string(path.join("cmdline"))
                                .unwrap_or_default()
                                .into(),
                        )]
                        .into(),
                    ),
                })
                .collect(),
        },
    );

    SimpleMetricSet { families }
}

#[tokio::main]
async fn main() {
    let metrics = generate_metrics();

    let plugin = MetricsPlugin::new("xcp-metrics-plugin-procfs", metrics.clone().into())
        .await
        .unwrap();

    // Expose protocol v2
    loop {
        // Fetch and push new metrics.
        plugin.update(generate_metrics().into()).await.unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }
}
