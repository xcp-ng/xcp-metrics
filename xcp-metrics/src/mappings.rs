//! Protocol v2 to protocol v3 predefined [CustomMapping]s.
use std::collections::HashMap;

use maplit::hashmap;
use xcp_metrics_common::utils::mapping::CustomMapping;

pub fn default_mappings() -> HashMap<Box<str>, CustomMapping> {
    hashmap! {
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
    }
}
