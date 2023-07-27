use std::{borrow::Cow, time::Instant};

use xcp_metrics_common::rrdd::protocol_common::{
    DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue,
};

use xenctrl::XenControl;
use xenctrl_sys::xen_sysctl_cpuinfo_t;

use super::{XenMetric, XenMetricsShared, XEN_PAGE_SIZE};

pub struct PCpuTime {
    cpu_index: usize,
    current_info: Option<(xen_sysctl_cpuinfo_t, Instant)>,
    previous_info: Option<(xen_sysctl_cpuinfo_t, Instant)>,
}

impl PCpuTime {
    pub fn new(cpu_index: usize) -> Self {
        Self {
            cpu_index,
            current_info: None,
            previous_info: None,
        }
    }
}

impl XenMetric for PCpuTime {
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata> {
        Ok(DataSourceMetadata {
            description: format!("Physical cpu usage for cpu {}", self.cpu_index).into(),
            units: "".into(),
            ds_type: DataSourceType::Gauge,
            value: DataSourceValue::Float(0.0),
            min: 0.0,
            max: 1.0,
            owner: DataSourceOwner::Host,
            default: true,
        })
    }

    fn update(&mut self, shared: &XenMetricsShared, _: &XenControl) -> bool {
        if let Some(cpuinfo) = shared.cpuinfos.get(self.cpu_index) {
            self.previous_info = self.current_info;
            self.current_info.replace((*cpuinfo, Instant::now()));

            true
        } else {
            false
        }
    }

    fn get_value(&self) -> DataSourceValue {
        match (self.current_info, self.previous_info) {
            (Some((current, current_instant)), Some((previous, previous_instant))) => {
                DataSourceValue::Float(
                    // Compute busy ratio over time.
                    1.0 - (((current.idletime - previous.idletime) as f64)
                        / 1.0e9
                        / current_instant
                            .duration_since(previous_instant)
                            .as_secs_f64()),
                )
            }
            (Some(_), None) => DataSourceValue::Float(0.0),
            (None, _) => DataSourceValue::Undefined,
        }
    }

    fn get_name(&self) -> Cow<str> {
        format!("cpu{}", self.cpu_index).into()
    }
}

#[derive(Default)]
pub struct MemoryTotal {
    memory_total: Option<i64>,
}

impl XenMetric for MemoryTotal {
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata> {
        Ok(DataSourceMetadata {
            description: "Total amount of memory in the host".into(),
            units: "KiB".into(),
            ds_type: DataSourceType::Gauge,
            value: DataSourceValue::Int64(0),
            min: 0.0,
            max: f32::INFINITY,
            owner: DataSourceOwner::Host,
            default: true,
        })
    }

    fn update(&mut self, shared: &XenMetricsShared, _: &XenControl) -> bool {
        self.memory_total = Some(
            shared.physinfo.map_or(0, |physinfo| physinfo.total_pages) as i64
                * XEN_PAGE_SIZE as i64
                / 1024,
        );

        true
    }

    fn get_value(&self) -> DataSourceValue {
        if let Some(count) = self.memory_total {
            DataSourceValue::Int64(count)
        } else {
            DataSourceValue::Undefined
        }
    }

    fn get_name(&self) -> Cow<str> {
        Cow::Borrowed("memory_total_kib")
    }
}

#[derive(Default)]
pub struct MemoryFree {
    memory_free: Option<i64>,
}

impl XenMetric for MemoryFree {
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata> {
        Ok(DataSourceMetadata {
            description: "Total amount of free memory".into(),
            units: "KiB".into(),
            ds_type: DataSourceType::Gauge,
            value: DataSourceValue::Int64(0),
            min: 0.0,
            max: f32::INFINITY,
            owner: DataSourceOwner::Host,
            default: true,
        })
    }

    fn update(&mut self, shared: &XenMetricsShared, _: &XenControl) -> bool {
        self.memory_free = Some(
            shared.physinfo.map_or(0, |physinfo| physinfo.free_pages) as i64 * XEN_PAGE_SIZE as i64
                / 1024,
        );

        true
    }

    fn get_value(&self) -> DataSourceValue {
        if let Some(value) = self.memory_free {
            DataSourceValue::Int64(value)
        } else {
            DataSourceValue::Undefined
        }
    }

    fn get_name(&self) -> Cow<str> {
        Cow::Borrowed("memory_free_kib")
    }
}
