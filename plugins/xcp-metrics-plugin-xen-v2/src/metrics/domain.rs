use std::{borrow::Cow, time::Instant};

use xcp_metrics_common::rrdd::protocol_common::{
    DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue,
};
use xenctrl::XenControl;
use xenctrl_sys::{xc_dominfo_t, xc_vcpuinfo_t};

use crate::XenMetric;

use super::{XenMetricsShared, XEN_PAGE_SIZE};

pub struct VCpuTime {
    vcpuid: u32,
    domid: u32,
    dom_uuid: uuid::Uuid,
    current_vcpu: Option<(xc_vcpuinfo_t, Instant)>,
    previous_vcpu: Option<(xc_vcpuinfo_t, Instant)>,
    name: Box<str>,
}

impl VCpuTime {
    pub fn new(vcpuid: u32, domid: u32, dom_uuid: uuid::Uuid) -> Self {
        Self {
            vcpuid,
            domid,
            dom_uuid,
            current_vcpu: None,
            previous_vcpu: None,
            name: format!("dom{domid}_vcpu{vcpuid}").into(),
        }
    }
}

impl XenMetric for VCpuTime {
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata> {
        Ok(DataSourceMetadata {
            description: format!("vCPU{} usage", self.vcpuid).into(),
            units: "".into(),
            ds_type: DataSourceType::Gauge,
            value: DataSourceValue::Float(0.0),
            min: 0.0,
            max: 1.0,
            owner: DataSourceOwner::VM(self.dom_uuid),
            default: true,
        })
    }

    fn update(&mut self, _: &XenMetricsShared, xc: &XenControl) -> bool {
        match xc.vcpu_getinfo(self.domid, self.vcpuid) {
            Ok(info) => {
                self.previous_vcpu = self.current_vcpu;
                self.current_vcpu.replace((info, Instant::now()));
                true
            }
            Err(e) => {
                eprintln!("vcpu_getinfo: {e}");
                false
            }
        }
    }

    fn get_value(&self) -> DataSourceValue {
        match (self.current_vcpu, self.previous_vcpu) {
            (Some(_), None) => DataSourceValue::Float(0.0),
            (Some((current, current_instant)), Some((previous, previous_instant))) => {
                // xcp-rrdd: Workaround for Xen leaking the flag XEN_RUNSTATE_UPDATE; using a mask of its complement ~(1 << 63)
                // Then convert from nanoseconds to seconds
                let cputime = (current.cpu_time & !(1u64 << 63)) as f64 / 1.0e9;
                // Do the same for previous cpu time.
                let previous_cputime = (previous.cpu_time & !(1u64 << 63)) as f64 / 1.0e9;

                DataSourceValue::Float(
                    (cputime - previous_cputime)
                        / current_instant
                            .duration_since(previous_instant)
                            .as_secs_f64(),
                )
            }
            (None, _) => DataSourceValue::Undefined,
        }
    }

    fn get_name(&self) -> Cow<str> {
        Cow::Borrowed(&self.name)
    }
}

pub struct DomainMemory {
    domid: u32,
    dom_uuid: uuid::Uuid,
    name: Box<str>,
    dominfo: Option<xc_dominfo_t>,
}

impl DomainMemory {
    pub fn new(domid: u32, dom_uuid: uuid::Uuid) -> Self {
        Self {
            dom_uuid,
            domid,
            name: format!("dom{domid}_memory").into(),
            dominfo: None,
        }
    }
}

impl XenMetric for DomainMemory {
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata> {
        Ok(DataSourceMetadata {
            description: "Memory currently allocated to VM".into(),
            units: "bytes".into(),
            ds_type: DataSourceType::Gauge,
            value: DataSourceValue::Int64(0),
            min: 0.0,
            max: f32::INFINITY,
            owner: DataSourceOwner::VM(self.dom_uuid),
            default: true,
        })
    }

    fn update(&mut self, shared: &XenMetricsShared, _: &XenControl) -> bool {
        if let Some(&info) = shared.dominfos.get(self.domid as usize) {
            self.dominfo.replace(info);
            true
        } else {
            false
        }
    }

    fn get_value(&self) -> DataSourceValue {
        self.dominfo.map_or_else(
            || DataSourceValue::Undefined,
            |info| DataSourceValue::Int64((info.nr_pages * XEN_PAGE_SIZE as u64) as i64),
        )
    }

    fn get_name(&self) -> Cow<str> {
        Cow::Borrowed(&self.name)
    }
}
