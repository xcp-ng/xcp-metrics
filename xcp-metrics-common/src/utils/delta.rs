//! Utilities to track changes between two metric sets.

// TODO: Consider supporting changes in metric families metadata ?

use std::{
    borrow::Cow,
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
    pub added_metrics: Vec<(&'a str, &'a Metric)>,

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
#[derive(Clone, Default, Debug)]
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
                  return Some(*uuid)
              };

                // Check for metric existence in family.
                // NOTE: As UUID is random due to conversion between raw OpenMetrics and xcp-metrics
                //       structure, we can't rely on it, and must use labels check existence.
                if !family
                    .metrics
                    .iter()
                    .any(|(_, metric)| labels == &metric.labels)
                {
                    Some(*uuid)
                } else {
                    None
                }
            })
            .collect();

        // Check for added metrics.
        let added_metrics = metrics_set
            .families
            .iter()
            // Combine family name with each family metric.
            .flat_map(|(name, family)| iter::zip(iter::repeat(name), family.metrics.iter()))
            // Only consider metrics we don't have, and strip uuid.
            .filter_map(|(name, (_, metric))| {
                // Due to contains_key expecting a tuple, we need to provide it a proper tuple (by cloning).
                // TODO: Find a better solution than cloning.
                if !self
                    .metrics_map
                    .contains_key(&(name.clone(), metric.labels.clone()))
                {
                    // We don't have the metric.
                    Some((name.as_ref(), metric))
                } else {
                    None
                }
            })
            .collect();

        // Check for families that doesn't exist anymore.
        let orphaned_families = self
            .families
            .iter()
            .filter(|family| !metrics_set.families.contains_key(*family))
            .map(|name| name.clone())
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
            .retain(|name| !delta.orphaned_families.contains(&Cow::Borrowed(name)));

        // Add new families
        delta.added_families.iter().for_each(|(name, _)| {
            self.families.insert(name.to_string().into());
        });

        // Add new metrics
        delta.added_metrics.iter().for_each(|(family, metrics)| {
            let uuid = uuid::Uuid::new_v4();

            self.metrics_map
                .insert((family.to_string().into(), metrics.labels.clone()), uuid);
        });
    }
}

/*
impl From<MetricSet> for MetricSetModel {
    fn from(value: MetricSet) -> Self {}
}
*/
