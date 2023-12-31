mod metrics;

use clap::Parser;
use maplit::hashmap;
use metrics::{discover_xen_metrics, XenMetric, XenMetricsShared};
use std::{collections::HashMap, path::PathBuf, rc::Rc};

use xcp_metrics_common::utils::mapping::CustomMapping;
use xcp_metrics_plugin_common::{
    plugin::{run_hybrid, XcpPlugin},
    protocol_v3::utils::SimpleMetricSet,
};

/// xcp-metrics Xen plugin.
#[derive(Clone, Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Logging level
    #[arg(short, long, default_value_t = tracing::Level::INFO)]
    log_level: tracing::Level,

    /// Target daemon path.
    #[arg(short, long)]
    target: Option<PathBuf>,

    /// Used protocol
    #[arg(short, long)]
    protocol: Option<u32>,
}

struct XenPlugin {
    shared: XenMetricsShared,
    sources: Box<[Box<dyn XenMetric>]>,
}

impl XcpPlugin for XenPlugin {
    fn update(&mut self) {
        self.shared.update()
    }

    fn generate_metrics(&mut self) -> SimpleMetricSet {
        SimpleMetricSet {
            families: self
                .sources
                .iter_mut()
                .filter_map(|source| source.get_family(&self.shared))
                .map(|(name, family)| (name.into_string(), family))
                .collect(),
        }
    }

    fn get_name(&self) -> &str {
        "xcp-metrics-plugin-xen"
    }

    fn get_mappings(&self) -> Option<HashMap<Box<str>, CustomMapping>> {
        Some(hashmap! {
            "cpu-cstate".into() => CustomMapping {
                pattern: "cpu{id}-C{state}".into(),
                min: 0.0,
                max: f32::INFINITY,
                default: true,
            },
            "cpu-pstate".into() => CustomMapping {
                pattern: "cpu{id}-P{state}".into(),
                min: 0.0,
                max: f32::INFINITY,
                default: true,
            },
            "cpu".into() => CustomMapping {
                pattern: "cpu{id}".into(),
                min: 0.0,
                max: 1.0,
                default: true,
            },
            "cpu-freq".into() => CustomMapping {
                pattern: "CPU{id}-avg-freq".into(),
                min: 0.0,
                max: f32::INFINITY,
                default: true
            },
        })
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

    let xc = Rc::new(xenctrl::XenControl::default().unwrap());

    let plugin = XenPlugin {
        sources: discover_xen_metrics(xc.clone()),
        shared: XenMetricsShared::new(xc),
    };

    run_hybrid(plugin, args.target.as_deref(), args.protocol).await;
}
