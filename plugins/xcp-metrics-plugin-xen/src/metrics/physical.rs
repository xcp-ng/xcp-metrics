use std::{borrow::Cow, mem::MaybeUninit, rc::Rc, time::Instant};

use xcp_metrics_common::rrdd::protocol_common::{
    DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue,
};
use xenctrl::XenControl;
use xenctrl_sys::{xc_physinfo_t, xen_sysctl_cpuinfo_t};

use crate::update_once::{Updatable, UpdateOnce};

use super::{XenMetric, XEN_PAGE_SIZE};

/// A shared cpuinfo slice.
pub struct SharedPCpuSlice {
    xc: Rc<XenControl>,
    buffer: Box<[MaybeUninit<xen_sysctl_cpuinfo_t>]>,
    initialized_len: Option<usize>,
}

impl Updatable for SharedPCpuSlice {
    fn update(&mut self) {
        let slice = self.xc.get_cpuinfo(&mut self.buffer).unwrap();
        self.initialized_len.replace(slice.len());
    }
}

impl<'a> SharedPCpuSlice {
    pub fn new(xc: Rc<XenControl>, pcpu_count: usize) -> Self {
        Self {
            xc,
            buffer: vec![MaybeUninit::zeroed(); pcpu_count].into_boxed_slice(),
            initialized_len: None,
        }
    }

    pub fn get_slice(&'a self) -> Option<&'a [xen_sysctl_cpuinfo_t]> {
        self.initialized_len
            .map(|len| unsafe { std::slice::from_raw_parts(self.buffer.as_ptr() as _, len) })
    }
}

pub struct SharedPhysInfo {
    xc: Rc<XenControl>,
    physinfo: Option<xc_physinfo_t>,
}

impl SharedPhysInfo {
    pub fn new(xc: Rc<XenControl>) -> Self {
        Self { xc, physinfo: None }
    }
}

impl Updatable for SharedPhysInfo {
    fn update(&mut self) {
        self.physinfo.replace(self.xc.physinfo().unwrap());
    }
}

pub struct PCpuTime {
    cpu_index: usize,
    slice: Rc<UpdateOnce<SharedPCpuSlice>>,
    current_info: Option<(xen_sysctl_cpuinfo_t, Instant)>,
    previous_info: Option<(xen_sysctl_cpuinfo_t, Instant)>,
}

impl PCpuTime {
    pub fn new(cpu_index: usize, slice: Rc<UpdateOnce<SharedPCpuSlice>>) -> Self {
        Self {
            cpu_index,
            slice,
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

    fn update(&mut self, token: uuid::Uuid) -> bool {
        self.slice.update(token);

        match self
            .slice
            .borrow()
            .get_slice()
            .and_then(|infos| infos.get(self.cpu_index))
        {
            Some(cpuinfo) => {
                self.previous_info = self.current_info;
                self.current_info.replace((*cpuinfo, Instant::now()));
                true
            }
            None => false,
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

pub struct MemoryTotal(Rc<UpdateOnce<SharedPhysInfo>>);

impl MemoryTotal {
    pub fn new(physinfo: Rc<UpdateOnce<SharedPhysInfo>>) -> Self {
        Self(physinfo)
    }
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

    fn update(&mut self, token: uuid::Uuid) -> bool {
        self.0.update(token);

        true
    }

    fn get_value(&self) -> DataSourceValue {
        match self.0.borrow().physinfo {
            Some(physinfo) => {
                DataSourceValue::Int64(physinfo.total_pages as i64 * XEN_PAGE_SIZE as i64 / 1024)
            }
            None => DataSourceValue::Undefined,
        }
    }

    fn get_name(&self) -> Cow<str> {
        Cow::Borrowed("memory_total_kib")
    }
}

pub struct MemoryFree(Rc<UpdateOnce<SharedPhysInfo>>);

impl MemoryFree {
    pub fn new(physinfo: Rc<UpdateOnce<SharedPhysInfo>>) -> Self {
        Self(physinfo)
    }
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

    fn update(&mut self, token: uuid::Uuid) -> bool {
        self.0.update(token);

        true
    }

    fn get_value(&self) -> DataSourceValue {
        match self.0.borrow().physinfo {
            Some(physinfo) => {
                DataSourceValue::Int64(physinfo.free_pages as i64 * XEN_PAGE_SIZE as i64 / 1024)
            }
            None => DataSourceValue::Undefined,
        }
    }

    fn get_name(&self) -> Cow<str> {
        Cow::Borrowed("memory_free_kib")
    }
}
