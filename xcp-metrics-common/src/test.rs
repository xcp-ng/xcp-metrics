//! Protocol v3 tests

use std::iter;

use crate::{
    metrics::{Label, Metric, MetricFamily, MetricSet, MetricType, MetricValue, NumberValue},
    utils::delta::MetricSetModel,
};

#[cfg(test)]
pub(crate) fn make_test_metrics_set() -> MetricSet {
    MetricSet {
        families: [
            (
                "test".into(),
                MetricFamily {
                    reference_count: 1,
                    metric_type: MetricType::Gauge,
                    unit: "unit".into(),
                    help: "help".into(),
                    metrics: [
                        (
                            uuid::Uuid::new_v4(),
                            Metric {
                                labels: vec![Label {
                                    name: "test".into(),
                                    value: "test".into(),
                                }]
                                .into(),
                                value: MetricValue::Gauge(NumberValue::Int64(1)),
                            },
                        ),
                        (
                            uuid::Uuid::new_v4(),
                            Metric {
                                labels: vec![].into(),
                                value: MetricValue::Gauge(NumberValue::Int64(1)),
                            },
                        ),
                    ]
                    .into_iter()
                    .collect(),
                },
            ),
            (
                "test2".into(),
                MetricFamily {
                    reference_count: 1,
                    metric_type: MetricType::Gauge,
                    unit: "unit".into(),
                    help: "help".into(),
                    metrics: [(
                        uuid::Uuid::new_v4(),
                        Metric {
                            labels: vec![Label {
                                name: "owner".into(),
                                value: uuid::Uuid::new_v4().as_hyphenated().to_string().into(),
                            }]
                            .into(),
                            value: MetricValue::Gauge(NumberValue::Int64(1)),
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            ),
            (
                "tes3".into(),
                MetricFamily {
                    reference_count: 1,
                    metric_type: MetricType::Gauge,
                    unit: "unit".into(),
                    help: "help".into(),
                    metrics: [(
                        uuid::Uuid::new_v4(),
                        Metric {
                            labels: vec![].into(),
                            value: MetricValue::Gauge(NumberValue::Int64(1)),
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            ),
        ]
        .into_iter()
        .collect(),
    }
}

fn assert_metrics_set_equals(a: &MetricSet, b: &MetricSet) {
    let metrics_model = MetricSetModel::from(a);
    let delta = metrics_model.compute_delta(&b);

    assert!(delta.added_families.is_empty());
    assert!(delta.added_metrics.is_empty());
    assert!(delta.removed_metrics.is_empty());

    a.families
        .iter()
        .flat_map(|(family_name, family)| {
            iter::zip(iter::repeat(family_name), family.metrics.iter())
        })
        .for_each(|(name, (_, metric))| {
            // Check for the same metric on b.
            if !b.families[name]
                .metrics
                .iter()
                .any(|(_, b_metric)| b_metric == metric)
            {
                panic!("Missing matching metric for {metric:?}");
            }
        })
}
