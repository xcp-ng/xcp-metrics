//! Protocol v3 tests

use std::{iter, time::SystemTime};

use crate::{
    metrics::{
        Label, Metric, MetricFamily, MetricPoint, MetricSet, MetricType, MetricValue, NumberValue,
    },
    utils::delta::MetricSetModel,
};

#[cfg(test)]
pub(crate) fn make_test_metrics_set() -> MetricSet {
    MetricSet {
        families: [
            (
                "test".into(),
                MetricFamily {
                    metric_type: MetricType::Gauge,
                    unit: "unit".into(),
                    help: "help".into(),
                    metrics: [
                        (
                            uuid::Uuid::new_v4(),
                            Metric {
                                labels: vec![Label("test".into(), "test".into())].into(),
                                metrics_point: vec![MetricPoint {
                                    value: MetricValue::Gauge(NumberValue::Int64(1)),
                                    timestamp: SystemTime::now(),
                                }]
                                .into(),
                            },
                        ),
                        (
                            uuid::Uuid::new_v4(),
                            Metric {
                                labels: vec![].into(),
                                metrics_point: vec![MetricPoint {
                                    value: MetricValue::Gauge(NumberValue::Int64(1)),
                                    timestamp: SystemTime::now(),
                                }]
                                .into(),
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
                    metric_type: MetricType::Gauge,
                    unit: "unit".into(),
                    help: "help".into(),
                    metrics: [(
                        uuid::Uuid::new_v4(),
                        Metric {
                            labels: vec![Label(
                                "owner".into(),
                                uuid::Uuid::new_v4().as_hyphenated().to_string().into(),
                            )]
                            .into(),
                            metrics_point: vec![MetricPoint {
                                value: MetricValue::Gauge(NumberValue::Int64(1)),
                                timestamp: SystemTime::now(),
                            }]
                            .into(),
                        },
                    )]
                    .into_iter()
                    .collect(),
                },
            ),
            (
                "tes3".into(),
                MetricFamily {
                    metric_type: MetricType::Gauge,
                    unit: "unit".into(),
                    help: "help".into(),
                    metrics: [(
                        uuid::Uuid::new_v4(),
                        Metric {
                            labels: vec![].into(),
                            metrics_point: vec![MetricPoint {
                                value: MetricValue::Gauge(NumberValue::Int64(1)),
                                timestamp: SystemTime::now(),
                            }]
                            .into(),
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

mod protocol_v3 {
    use std::time::{Duration, SystemTime};

    use crate::protocol_v3::{
        self, generate_v3, generate_v3_async, parse_v3, parse_v3_async, ProtocolV3Error,
    };

    use super::{assert_metrics_set_equals, make_test_metrics_set};

    #[test]
    fn header() {
        let metrics_set = make_test_metrics_set();

        // Generate raw payload.
        let mut buffer = vec![];
        generate_v3(&mut buffer, None, metrics_set.clone()).unwrap();

        let (_, metrics_readed) = protocol_v3::parse_v3(&mut buffer.as_slice()).unwrap();

        // We can't lazily compare them as xcp-metrics metrics has some additional informations
        // (like internal uuid) that are randomly generated when parsing from OpenMetrics.

        // Compare readed and original.
        assert_metrics_set_equals(&metrics_set, &metrics_readed);
    }

    #[test]
    fn invalid_header() {
        // Empty header is invalid.
        let invalid_header = [0u8; 28];

        assert!(matches!(
            parse_v3(&mut invalid_header.as_slice()),
            Err(ProtocolV3Error::InvalidHeader)
        ));
    }

    #[test]
    fn invalid_header_async() {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async {
                // Empty header is invalid.
                let invalid_header = [0u8; 28];

                assert!(matches!(
                    parse_v3_async(&mut invalid_header.as_slice()).await,
                    Err(ProtocolV3Error::InvalidHeader)
                ));
            });
    }

    #[test]
    fn invalid_checksum() {
        // Empty header is invalid.
        let mut dest = vec![];

        generate_v3(
            &mut dest,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(123456789)),
            crate::test::make_test_metrics_set(),
        )
        .unwrap();

        // mess with dest checksum
        dest.get_mut(12..16)
            .unwrap()
            .copy_from_slice(&[0xde, 0xad, 0xbe, 0xef]);

        assert!(matches!(
            parse_v3(&mut dest.as_slice()),
            Err(ProtocolV3Error::InvalidChecksum { .. })
        ));
    }

    #[test]
    fn invalid_checksum_async() {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async {
                // Empty header is invalid.
                let mut dest = vec![];

                generate_v3_async(
                    &mut dest,
                    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(123456789)),
                    crate::test::make_test_metrics_set(),
                )
                .await
                .unwrap();

                // mess with dest checksum
                dest.get_mut(12..16)
                    .unwrap()
                    .copy_from_slice(&[0xde, 0xad, 0xbe, 0xef]);

                assert!(matches!(
                    parse_v3_async(&mut dest.as_slice()).await,
                    Err(ProtocolV3Error::InvalidChecksum { .. })
                ));
            });
    }

    #[test]
    fn invalid_openmetrics() {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(async {
                // Empty header is invalid.
                let mut dest = vec![];

                generate_v3_async(
                    &mut dest,
                    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(123456789)),
                    crate::test::make_test_metrics_set(),
                )
                .await
                .unwrap();

                // mess with dest checksum
                dest.get_mut(12..16)
                    .unwrap()
                    .copy_from_slice(&[0xde, 0xad, 0xbe, 0xef]);

                assert!(matches!(
                    parse_v3_async(&mut dest.as_slice()).await,
                    Err(ProtocolV3Error::InvalidChecksum { .. })
                ));
            });
    }
}
