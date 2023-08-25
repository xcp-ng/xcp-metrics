/*!
Utilities to track changes between two [MetricSet].

# Usage
```rust
use std::time::SystemTime;
use maplit::hashmap;
use xcp_metrics_common::{
    metrics::{
        Metric, MetricFamily, MetricPoint, MetricSet, MetricType, MetricValue, NumberValue,
    },
    utils::delta::MetricSetModel,
};

// Generate a test metric family.
let (test_metric_uuid, test_metric) = (
    uuid::Uuid::new_v4(),
    Metric {
        labels: [].into(),
        metrics_point: [MetricPoint {
            value: MetricValue::Gauge(NumberValue::Double(42.0)),
            timestamp: SystemTime::now(),
        }]
        .into(),
    },
);
let test_family = MetricFamily {
    metric_type: MetricType::Gauge,
    unit: "test".into(),
    help: "test metric family".into(),
    metrics: hashmap! { test_metric_uuid => test_metric.clone() },
};

// Create a empty metric set and a metric set with test_family.
let set1 = MetricSet::default();
let set2 = MetricSet {
    families: hashmap! { "test_family".into() => test_family.clone() },
};

// Create related models.
let mut set1_model = MetricSetModel::from(set1.clone());
let set2_model = MetricSetModel::from(set2.clone());

// We can also create a empty model.
let empty_model = MetricSetModel::default();

// Should be the same as set1_model, as set1 is empty.
assert_eq!(set1_model, empty_model);

// Compute the differences between set1 and set2.
let delta1 = set1_model.compute_delta(&set2);

// test_family and its related metric has been added.
assert_eq!(&delta1.added_families, &[("test_family", &test_family)]);
assert_eq!(
    &delta1.added_metrics,
    &[("test_family", &test_metric, test_metric_uuid)]
);

// Nothing has been removed (empty set initially).
assert!(&delta1.orphaned_families.is_empty());
assert!(&delta1.removed_metrics.is_empty());

// Compute the differences between set2 and set1.
let delta2 = set2_model.compute_delta(&set1);

// Nothing has been added
assert!(delta2.added_families.is_empty());
assert!(delta2.added_metrics.is_empty());

// test_metric is removed
assert_eq!(&delta2.removed_metrics, &[test_metric_uuid]);
// test_family is now empty, thus 'orphaned'
assert_eq!(&delta2.orphaned_families, &["test_family".into()]);

// Update set1 model to match set2, to do this, apply delta1 (difference between set1 and set2).
set1_model.apply_delta(&delta1);

// set1 model now matches set2.
assert_eq!(set1_model, set2_model);
let delta_updated = set1_model.compute_delta(&set2);

assert!(delta_updated.added_families.is_empty());
assert!(delta_updated.added_metrics.is_empty());
assert!(delta_updated.orphaned_families.is_empty());
assert!(delta_updated.removed_metrics.is_empty());
```

# Note

This module doesn't rely on [uuid::Uuid] but rather on [Label]s to compute differences between
a [MetricSet] and the [MetricSetModel], which makes it usable to compare between [crate::openmetrics::MetricSet]
(which have randomized [uuid::Uuid]).
*/

// TODO: Consider supporting changes in metric families metadata ?

use std::{
    collections::{HashMap, HashSet},
    iter,
};

use crate::metrics::{Label, Metric, MetricFamily, MetricSet};

/// Summary of changes between a MetricSetModel and a MetricSet (ignoring MetricSet UUIDs)
/// Borrows used MetricSet.
#[derive(Debug, Default)]
pub struct MetricSetDelta<'a> {
    /// New added families (name only)
    pub added_families: Vec<(&'a str, &'a MetricFamily)>,

    /// Changed family metadata
    // changed_families: Vec<&'a str>,

    /// Metrics that no longer contain a family.
    /// In case they reappears, they will need to be registered again.
    pub orphaned_families: Vec<Box<str>>,

    // Added metrics
    pub added_metrics: Vec<(&'a str, &'a Metric, uuid::Uuid)>,

    // Removed metrics
    pub removed_metrics: Vec<uuid::Uuid>,
    // Updated metrics
    // Currently consider updating all metrics.
    // TODO: Do some testing/benchmark about this. Maybe force-update
    //       all metrics other than Drop-heavy structures like Info,
    //       StateSet, Summary, Histogram, ...
    //
    // updated: Vec<UpdateMetrics>,
}

/// A MetricSet model, used to compute MetricSet delta.
#[derive(Clone, Default, Debug, PartialEq)]
pub struct MetricSetModel {
    /// Track metrics per family and labels set.
    pub metrics_map: HashMap<(Box<str>, Box<[Label]>), uuid::Uuid>,
    pub families: HashSet<Box<str>>,
}

impl MetricSetModel {
    /// Compute variation between metrics_set and current model.
    pub fn compute_delta<'a>(&'_ self, metrics_set: &'a MetricSet) -> MetricSetDelta<'a> {
        // Check for new families.
        let added_families = metrics_set
            .families
            .iter()
            .filter(|(name, _)| !self.families.contains(*name))
            .map(|(name, family)| (name.as_ref(), family))
            .collect();

        // Check for removed metrics.
        let removed_metrics = self
            .metrics_map
            .iter()
            .filter_map(|((name, labels), uuid)| {
                let Some(family) = metrics_set.families.get(name) else {
                    // Related family doesn't exist anymore, so do metric.
                    return Some(*uuid);
                };

                // Check for metric existence in family.
                // NOTE: As UUID is random due to conversion between raw OpenMetrics and xcp-metrics
                //       structure, we can't rely on it, and must use labels to check existence.
                (!family
                    .metrics
                    .iter()
                    .any(|(_, metric)| labels == &metric.labels))
                .then_some(*uuid)
            })
            .collect();

        // Check for added metrics.
        let added_metrics = metrics_set
            .families
            .iter()
            // Combine family name with each family metric.
            .flat_map(|(name, family)| iter::zip(iter::repeat(name), family.metrics.iter()))
            // Only consider metrics we don't have, and generate a new proper UUID.
            .filter_map(|(name, (&uuid, metric))| {
                // Due to contains_key expecting a tuple, we need to provide it a proper tuple (by cloning).
                // TODO: Find a better solution than cloning.
                (!self
                    .metrics_map
                    .contains_key(&(name.clone(), metric.labels.clone())))
                .then(|| (name.as_ref(), metric, uuid))
            })
            .collect();

        // Check for families that doesn't exist anymore.
        let orphaned_families = self
            .families
            .iter()
            .filter(|family| !metrics_set.families.contains_key(*family))
            .cloned()
            .collect();

        MetricSetDelta {
            added_families,
            orphaned_families,
            added_metrics,
            removed_metrics,
        }
    }

    pub fn apply_delta(&mut self, delta: &MetricSetDelta) {
        // Update mapping, only keep those non-removed
        self.metrics_map
            .retain(|_, uuid| !delta.removed_metrics.contains(uuid));

        // Remove orphaned families.
        self.families
            .retain(|name| !delta.orphaned_families.contains(name));

        // Add new families
        delta.added_families.iter().for_each(|(name, _)| {
            self.families.insert(name.to_string().into());
        });

        // Add new metrics
        delta
            .added_metrics
            .iter()
            .for_each(|(family, metrics, uuid)| {
                self.metrics_map
                    .insert((family.to_string().into(), metrics.labels.clone()), *uuid);
            });
    }
}

impl From<MetricSet> for MetricSetModel {
    fn from(value: MetricSet) -> Self {
        Self::from(&value)
    }
}

impl From<&MetricSet> for MetricSetModel {
    fn from(set: &MetricSet) -> Self {
        let families = set.families.keys().cloned().collect();

        let metrics_map = set
            .families
            .iter()
            .flat_map(|(name, family)| iter::zip(iter::repeat(name), &family.metrics))
            .map(|(name, (&uuid, metric))| ((name.clone(), metric.labels.clone()), uuid))
            .collect();

        MetricSetModel {
            metrics_map,
            families,
        }
    }
}
