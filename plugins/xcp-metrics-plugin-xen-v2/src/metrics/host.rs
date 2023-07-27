use std::borrow::Cow;

use xcp_metrics_common::rrdd::protocol_common::{
    DataSourceMetadata, DataSourceOwner, DataSourceType, DataSourceValue,
};
use xenctrl::XenControl;

use crate::XenMetric;

use super::XenMetricsShared;

#[derive(Default)]
pub struct LoadAvg(f64);

impl XenMetric for LoadAvg {
    fn generate_metadata(&self) -> anyhow::Result<DataSourceMetadata> {
        Ok(DataSourceMetadata {
            description: "Domain0 loadavg".into(),
            units: "".into(),
            ds_type: DataSourceType::Gauge,
            value: DataSourceValue::Float(0.0),
            min: f32::NEG_INFINITY,
            max: f32::INFINITY,
            owner: DataSourceOwner::Host,
            default: true,
        })
    }

    fn update(&mut self, _: &XenMetricsShared, _: &XenControl) -> bool {
        let proc_loadavg =
            std::fs::read_to_string("/proc/loadavg").expect("Unable to read /proc/loadavg");

        self.0 = proc_loadavg
            .split_once(' ')
            .expect("No first element in /proc/loadavg ?")
            .0
            .parse()
            .expect("First part of /proc/loadavg not a number ?");

        true
    }

    fn get_value(&self) -> DataSourceValue {
        DataSourceValue::Float(self.0)
    }

    fn get_name(&self) -> Cow<str> {
        Cow::Borrowed("loadavg")
    }
}
