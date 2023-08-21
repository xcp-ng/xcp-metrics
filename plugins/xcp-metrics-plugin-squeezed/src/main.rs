use std::time::Duration;

use maplit::hashmap;
use tokio::time;
use xcp_metrics_common::metrics::{MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::protocol_v3::{
    utils::{SimpleMetric, SimpleMetricFamily, SimpleMetricSet},
    MetricsPlugin,
};
use xenstore_rs::{XBTransaction, Xs, XsOpenFlags};

struct SqueezedInfo {
    /// Host memory reclaimed by squeezed.
    /// dynamic_max - target
    reclaimed: u32,
    /// Host memory that could be reclaimed by squeezed.
    /// target - dynamic_min
    reclaimed_max: u32,
}

impl SqueezedInfo {
    fn get(xs: &Xs) -> anyhow::Result<Self> {
        let (reclaimed, reclaimed_max) = xs
            // Get the list of domains.
            .directory(XBTransaction::Null, "/local/domain")?
            .iter()
            // Read all values, filtering out domains that have any value missing.
            .filter_map(|domid| {
                xs.read(
                    XBTransaction::Null,
                    format!("/local/domain/{domid}/memory/target").as_str(),
                )
                .ok()
                .zip(
                    xs.read(
                        XBTransaction::Null,
                        format!("/local/domain/{domid}/memory/dynamic-min").as_str(),
                    )
                    .ok(),
                )
                .zip(
                    xs.read(
                        XBTransaction::Null,
                        format!("/local/domain/{domid}/memory/dynamic-max").as_str(),
                    )
                    .ok(),
                )
            })
            // Parse values.
            .filter_map(|((target, dynamic_min), dynamic_max)| {
                target
                    .parse::<u32>()
                    .ok()
                    .zip(dynamic_min.parse::<u32>().ok())
                    .zip(dynamic_max.parse::<u32>().ok())
            })
            // Compute reclaimed and reclaimed_max based on iterator values.
            .fold(
                (0u32, 0u32),
                |(reclaimed, reclaimed_max), ((target, dynamic_min), dynamic_max)| {
                    (
                        (reclaimed + dynamic_max - target),
                        reclaimed_max + target - dynamic_min,
                    )
                },
            );

        Ok(SqueezedInfo {
            reclaimed,
            reclaimed_max,
        })
    }
}

fn generate_metrics(xs: &Xs) -> anyhow::Result<SimpleMetricSet> {
    let SqueezedInfo {
        reclaimed,
        reclaimed_max,
    } = SqueezedInfo::get(xs)?;

    Ok(SimpleMetricSet {
        families: hashmap! {
        "memory_reclaimed".into() =>
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "B".into(),
                help: "Host memory reclaimed by squeezed".into(),
                metrics: vec![SimpleMetric {
                    labels: vec![],
                    // KiB to Bytes
                    value: MetricValue::Gauge(NumberValue::Int64((reclaimed * 1024) as _)),
                }],
            },
        "memory_reclaimed_max".into() =>
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "B".into(),
                help: "Host memory that could be reclaimed by squeezed".into(),
                metrics: vec![SimpleMetric {
                    labels: vec![],
                    // KiB to Bytes
                    value: MetricValue::Gauge(NumberValue::Int64((reclaimed_max * 1024) as _)),
                }],
            }
        },
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let xs = Xs::new(XsOpenFlags::ReadOnly).map_err(|e| anyhow::anyhow!("{e}"))?;

    let plugin = MetricsPlugin::new(
        "xcp-metrics-plugin-squeezed",
        generate_metrics(&xs)?.into(),
        None,
    )
    .await?;

    loop {
        // Fetch and push new metrics.
        plugin.update(generate_metrics(&xs)?.into()).await.unwrap();

        time::sleep(Duration::from_secs(1)).await;
    }
}
