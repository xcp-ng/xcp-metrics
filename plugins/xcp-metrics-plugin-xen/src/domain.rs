use std::{borrow::Cow, rc::Rc};

use xcp_metrics_common::rrdd::protocol_common::{
    DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue,
};
use xenctrl::XenControl;
use xenctrl_sys::{xc_dominfo_t, xc_vcpuinfo_t};

use crate::{XenMetric, XEN_PAGE_SIZE};

pub struct VCpuTime {
    xc: Rc<XenControl>,
    vcpuid: u32,
    domid: u32,
    dom_uuid: uuid::Uuid,
    vcpu_info: Option<xc_vcpuinfo_t>,
    name: Box<str>,
}

impl VCpuTime {
    pub fn new(xc: Rc<XenControl>, vcpuid: u32, domid: u32, dom_uuid: uuid::Uuid) -> Self {
        Self {
            xc,
            vcpuid,
            domid,
            dom_uuid,
            vcpu_info: None,
            name: format!("dom{domid}_vcpu{vcpuid}").into(),
        }
    }
}

impl XenMetric for VCpuTime {
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata> {
        Ok(DataSourceMetadata {
            description: format!("vCPU{} usage", self.vcpuid).into(),
            units: "(fraction)".into(),
            ds_type: DataSourceType::Derive,
            value: DataSourceValue::Float(0.0),
            min: 0.0,
            max: 1.0,
            owner: DataSourceOwner::VM(self.dom_uuid),
            default: true,
        })
    }

    fn update(&mut self) -> bool {
        match self.xc.vcpu_getinfo(self.domid, self.vcpuid) {
            Ok(info) => {
                self.vcpu_info.replace(info);
                true
            }
            Err(e) => {
                eprintln!("vcpu_getinfo: {e}");
                false
            }
        }
    }

    fn get_value(&self) -> DataSourceValue {
        self.vcpu_info.map_or_else(
            || DataSourceValue::Undefined,
            |info| {
                // xcp-rrdd: Workaround for Xen leaking the flag XEN_RUNSTATE_UPDATE; using a mask of its complement ~(1 << 63)
                let mut cputime = (info.cpu_time & !(1u64 << 63)) as f64;
                // Convert from nanoseconds to seconds
                cputime /= 1.0e9;

                DataSourceValue::Float(cputime)
            },
        )
    }

    fn get_name(&self) -> Cow<str> {
        Cow::Borrowed(&self.name)
    }
}

pub struct DomainMemory {
    xc: Rc<XenControl>,
    domid: u32,
    dom_uuid: uuid::Uuid,
    name: Box<str>,
    dominfo: Option<xc_dominfo_t>,
}

impl DomainMemory {
    pub fn new(xc: Rc<XenControl>, domid: u32, dom_uuid: uuid::Uuid) -> Self {
        Self {
            xc,
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
            units: "B".into(),
            ds_type: DataSourceType::Gauge,
            value: DataSourceValue::Int64(0),
            min: 0.0,
            max: f32::INFINITY,
            owner: DataSourceOwner::VM(self.dom_uuid),
            default: true,
        })
    }

    fn update(&mut self) -> bool {
        match self.xc.domain_getinfo(self.domid) {
            Ok(Some(info)) => {
                self.dominfo.replace(info);
                true
            }
            Ok(None) => false,
            Err(e) => {
                eprintln!("DomainMemory: {e}");
                false
            }
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
