// TODO: Share buffers.

use std::{borrow::Cow, mem::MaybeUninit, rc::Rc};

use xcp_metrics_common::rrdd::protocol_common::{
    DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue,
};
use xenctrl::XenControl;
use xenctrl_sys::{xc_physinfo_t, xen_sysctl_cpuinfo_t};

use crate::update_once::{Updatable, UpdateOnce};

use super::XenMetric;

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

impl Updatable for SharedPhysInfo {
    fn update(&mut self) {
        self.physinfo.replace(self.xc.physinfo().unwrap());
    }
}

pub struct PCpuTime {
    cpu_index: usize,
    slice: Rc<UpdateOnce<SharedPCpuSlice>>,
    info: Option<xen_sysctl_cpuinfo_t>,
}

impl PCpuTime {
    pub fn new(cpu_index: usize, slice: Rc<UpdateOnce<SharedPCpuSlice>>) -> Self {
        Self {
            cpu_index,
            slice,
            info: None,
        }
    }
}

impl XenMetric for PCpuTime {
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata> {
        Ok(DataSourceMetadata {
            description: format!("Physical cpu usage for cpu {}", self.cpu_index).into(),
            units: "(fraction)".into(),
            ds_type: DataSourceType::Derive,
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
            .map(|infos| infos.get(self.cpu_index))
        {
            Some(Some(cpuinfo)) => {
                self.info.replace(*cpuinfo);
                true
            }
            _ => false,
        }
    }

    fn get_value(&self) -> DataSourceValue {
        match self.info {
            Some(info) => DataSourceValue::Float(1.0 - ((info.idletime as f64) / 1.0e9)),
            None => DataSourceValue::Undefined,
        }
    }

    fn get_name(&self) -> Cow<str> {
        format!("cpu{}", self.cpu_index).into()
    }
}
