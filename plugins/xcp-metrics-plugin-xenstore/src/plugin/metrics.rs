use enum_dispatch::enum_dispatch;
use xcp_metrics_common::metrics::{Metric, MetricValue, NumberValue};
use xenstore_rs::AsyncXs;

#[enum_dispatch]
pub(crate) trait MetricHandler {
    fn subpath(&self) -> &'static str;
    fn family_name(&self) -> &'static str;

    async fn read_metric(&self, xs: &impl AsyncXs, path: &str, subpath: &str) -> Option<Metric>;
}

#[derive(Default)]
pub struct MemInfoTotal;

impl MetricHandler for MemInfoTotal {
    fn subpath(&self) -> &'static str {
        "data/meminfo_total"
    }

    fn family_name(&self) -> &'static str {
        "xen_memory_usage_total"
    }

    async fn read_metric(&self, xs: &impl AsyncXs, path: &str, subpath: &str) -> Option<Metric> {
        if subpath != self.subpath() {
            return None;
        }

        let mut mem_total = xs.read(&path).await.ok()?.parse().ok()?;
        mem_total *= 1024; // KiB to bytes

        Some(Metric {
            labels: vec![].into_boxed_slice(),
            value: MetricValue::Gauge(NumberValue::Int64(mem_total)),
        })
    }
}

#[derive(Default)]
pub struct MemInfoFree;

impl MetricHandler for MemInfoFree {
    fn subpath(&self) -> &'static str {
        "data/meminfo_free"
    }

    fn family_name(&self) -> &'static str {
        "xen_memory_usage_free"
    }

    async fn read_metric(&self, xs: &impl AsyncXs, path: &str, subpath: &str) -> Option<Metric> {
        if subpath != self.subpath() {
            return None;
        }

        let mut mem_total = xs.read(&path).await.ok()?.parse().ok()?;
        mem_total *= 1024; // KiB to bytes

        Some(Metric {
            labels: vec![].into_boxed_slice(),
            value: MetricValue::Gauge(NumberValue::Int64(mem_total)),
        })
    }
}

#[enum_dispatch(MetricHandler)]
pub enum MetricHandlerEnum {
    MemInfoTotal(MemInfoTotal),
    MemInfoFree(MemInfoFree),
}
