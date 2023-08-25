#[cfg(test)]
mod test;

use std::collections::HashMap;

use clap::Parser;
use maplit::hashmap;

use xcp_metrics_common::metrics::{MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::{
    plugin::{run_hybrid, XcpPlugin},
    protocol_v3::utils::{SimpleMetric, SimpleMetricFamily, SimpleMetricSet},
    xenstore::{xs::{XBTransaction, Xs, XsOpenFlags}, read::XsRead},
};

/// xcp-metrics Squeezed plugin.
#[derive(Clone, Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Logging level
    #[arg(short, long, default_value_t = tracing::Level::INFO)]
    log_level: tracing::Level,

    /// Target daemon.
    #[arg(short, long, default_value_t = String::from("xcp-rrdd"))]
    target: String,

    /// Used protocol
    #[arg(short, long, default_value_t = 2)]
    protocol: u32,
}

#[derive(Debug, PartialEq)]
pub struct SqueezedInfo {
    /// Host memory reclaimed by squeezed.
    /// dynamic_max - target
    pub reclaimed: u32,
    /// Host memory that could be reclaimed by squeezed.
    /// target - dynamic_min
    pub reclaimed_max: u32,
}

impl SqueezedInfo {
    pub fn get<XS: XsRead>(xs: &XS) -> anyhow::Result<Self> {
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

pub struct SqueezedPlugin<XS: XsRead> {
    xs: XS,
}

impl<XS: XsRead> XcpPlugin for SqueezedPlugin<XS> {
    fn update(&mut self) {}

    fn generate_metrics(&mut self) -> SimpleMetricSet {
        let Ok(SqueezedInfo {
            reclaimed,
            reclaimed_max,
        }) = SqueezedInfo::get(&self.xs) else {
            // No data
            tracing::warn!("No /local/domain found");
            return SimpleMetricSet {
                families: hashmap!{},
            };
        };

        SimpleMetricSet {
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
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let text_subscriber = tracing_subscriber::fmt()
        .with_ansi(true)
        .with_max_level(args.log_level)
        .compact()
        .finish();

    tracing::subscriber::set_global_default(text_subscriber).unwrap();

    let xs = match Xs::new(XsOpenFlags::ReadOnly) {
        Ok(xs) => xs,
        Err(e) => {
            tracing::error!("Unable to initialize XenStore {e}");
            return;
        }
    };

    let plugin = SqueezedPlugin { xs };

    run_hybrid(
        plugin,
        HashMap::default(),
        "xcp-metrics-plugin-squeezed",
        Some(&args.target),
        args.protocol,
    )
    .await;
}
