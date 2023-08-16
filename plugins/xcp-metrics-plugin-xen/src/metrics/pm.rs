//! Power management metrics

use std::iter;

use xcp_metrics_common::metrics::{Label, MetricType, MetricValue, NumberValue};
use xcp_metrics_plugin_common::protocol_v3::utils::{SimpleMetric, SimpleMetricFamily};

use super::{XenMetric, XenMetricsShared};

#[derive(Default)]
pub struct CpuAvgFrequency;

impl XenMetric for CpuAvgFrequency {
    fn get_family(&mut self, shared: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)> {
        let metrics = shared
            .cpufreqs
            .iter()
            .enumerate()
            .map(|(cpuid, &frequency)| {
                SimpleMetric {
                    labels: [Label("id".into(), cpuid.to_string().into())].into(),
                    value: MetricValue::Gauge(NumberValue::Int64(frequency as i64)),
                }
                .into()
            })
            .collect();

        Some((
            "cpu-freq".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "MHz".into(),
                help: "Average frequency of CPU".into(),
                metrics,
            },
        ))
    }
}

#[derive(Default)]
pub struct CpuPState;

impl XenMetric for CpuPState {
    fn get_family(&mut self, shared: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)> {
        let metrics = shared
            .pstates
            .iter()
            .enumerate()
            .flat_map(|(cpuid, px_stat)| {
                iter::zip(iter::repeat(cpuid), px_stat.values.iter().enumerate())
            })
            .map(|(cpuid, (state, val))| SimpleMetric {
                labels: [
                    Label("id".into(), cpuid.to_string().into()),
                    Label("state".into(), state.to_string().into()),
                ]
                .into(),
                value: MetricValue::Gauge(NumberValue::Int64(val.residency as i64)),
            })
            .collect();

        Some((
            "cpu-pstate".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "MHz".into(),
                help: "P-State times of CPU".into(),
                metrics,
            },
        ))
    }
}

#[derive(Default)]
pub struct CpuCState;

impl XenMetric for CpuCState {
    fn get_family(&mut self, shared: &XenMetricsShared) -> Option<(Box<str>, SimpleMetricFamily)> {
        let metrics = shared
            .cstates
            .iter()
            .enumerate()
            .flat_map(|(cpuid, cx_stat)| {
                iter::zip(iter::repeat(cpuid), cx_stat.residencies.iter().enumerate())
            })
            .map(|(cpuid, (state, &val))| SimpleMetric {
                labels: [
                    Label("id".into(), cpuid.to_string().into()),
                    Label("state".into(), state.to_string().into()),
                ]
                .into(),
                value: MetricValue::Gauge(NumberValue::Int64(val as i64)),
            })
            .collect();

        Some((
            "cpu-cstate".into(),
            SimpleMetricFamily {
                metric_type: MetricType::Gauge,
                unit: "MHz".into(),
                help: "C-State times of CPU".into(),
                metrics,
            },
        ))
    }
}
