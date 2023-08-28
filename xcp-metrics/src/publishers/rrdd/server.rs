//! [RrddServer] implementation
use std::{
    collections::HashMap,
    iter,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{self, select, sync::mpsc, task::JoinHandle};

use xcp_metrics_common::{
    metrics::{Metric, MetricFamily, MetricSet, MetricType, MetricValue, NumberValue},
    rrdd::{
        protocol_common::{DataSourceMetadata, DataSourceOwner},
        rrd_updates::RrdXport,
    },
    utils::mapping::{CustomMapping, DefaultMapping, MetadataMapping},
};

use super::{entry::RrdEntry, Granuality, RrdXportFilter, RrdXportInfo};

use crate::hub::{HubPullResponse, HubPushMessage, PullMetrics};

/// Types of message to communicate with [RrddServer].
#[derive(Debug)]
pub enum RrddServerMessage {
    RequestRrdUpdates(RrdXportInfo, mpsc::Sender<anyhow::Result<RrdXport>>),
}

/// xcp-rrdd partially compatible server that stores the state of metrics over time.
///
/// See [super] for more information.
#[derive(Debug)]
pub struct RrddServer {
    receiver: mpsc::UnboundedReceiver<RrddServerMessage>,
    host_uuid: uuid::Uuid,
    mappings: HashMap<Box<str>, CustomMapping>,

    /// Map a UUID from hub's MetricSet with a RrdEntry.
    entry_database: HashMap<uuid::Uuid, RrdEntry>,

    latest_update: SystemTime,

    minute_update_counter: u32,
    latest_minute_update: SystemTime,

    hour_update_counter: u32,
    latest_hour_update: SystemTime,

    day_update_counter: u32,
    latest_day_update: SystemTime,
}

/// Get the owner part of the rrd name (i.e vm:UUID).
fn get_owner_part(metric: &Metric, host_uuid: uuid::Uuid) -> (String, DataSourceOwner) {
    metric
        .labels
        .iter()
        .filter(|l| l.0.as_ref() == "owner")
        .find_map(|l| {
            let parts: Vec<&str> = l.1.split_ascii_whitespace().take(2).collect();
            let (kind, uuid) = (parts.first()?, parts.get(1)?);

            let owner = DataSourceOwner::try_from(l.1.as_ref()).unwrap_or(DataSourceOwner::Host);

            Some((format!("{kind}:{uuid}"), owner))
        })
        .unwrap_or_else(|| {
            (
                format!("host:{}", host_uuid.as_hyphenated()),
                DataSourceOwner::Host,
            )
        })
}

impl RrddServer {
    pub fn new(
        mappings: HashMap<Box<str>, CustomMapping>,
    ) -> (Self, mpsc::UnboundedSender<RrddServerMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        (
            Self {
                receiver,
                host_uuid: uuid::Uuid::new_v4(),
                mappings,

                entry_database: HashMap::new(),

                latest_update: SystemTime::now(),

                day_update_counter: 0,
                latest_day_update: SystemTime::now(),

                hour_update_counter: 0,
                latest_hour_update: SystemTime::now(),

                minute_update_counter: 0,
                latest_minute_update: SystemTime::now(),
            },
            sender,
        )
    }

    fn to_name_v2(
        mappings: &HashMap<Box<str>, CustomMapping>,
        metric: &Metric,
        family: &MetricFamily,
        family_name: &str,
    ) -> Option<(Box<str>, DataSourceMetadata)> {
        if let Some(custom_mapping) = mappings.get(family_name) {
            custom_mapping.convert(family_name, family, metric)
        } else {
            DefaultMapping.convert(family_name, family, metric)
        }
    }

    pub async fn pull_metrics(
        &mut self,
        hub_channel: &mpsc::UnboundedSender<HubPushMessage>,
    ) -> anyhow::Result<Arc<MetricSet>> {
        let (tx, mut rx) = mpsc::unbounded_channel();
        hub_channel.send(HubPushMessage::PullMetrics(PullMetrics(tx)))?;

        let response = rx.recv().await.ok_or(anyhow::anyhow!("No response"))?;

        match response {
            HubPullResponse::Metrics(metrics) => Ok(metrics), //r => tracing::error!("Unsupported hub response: {r:?}"),
        }
    }

    /// Update all metrics
    pub fn update_metrics(&mut self, metrics: &MetricSet) {
        let do_update_minute = {
            self.minute_update_counter =
                (self.minute_update_counter + 1) % Granuality::Minute.get_five_seconds_interval();
            self.minute_update_counter == 0
        };

        let do_update_hour = {
            self.hour_update_counter =
                (self.hour_update_counter + 1) % Granuality::Hour.get_five_seconds_interval();
            self.hour_update_counter == 0
        };

        let do_update_day = {
            self.day_update_counter =
                (self.day_update_counter + 1) % Granuality::Day.get_five_seconds_interval();
            self.day_update_counter == 0
        };

        // TODO: Take in account removed metrics, as they will no longer exist in latest MetricSet.
        // Maybe use a MetricSetModel to track added metrics, and rely on entry_database to iterate ?
        metrics
            .families
            .iter()
            .filter(|(_, family)| {
                // Only consider gauge and counter metrics.
                matches!(family.metric_type, MetricType::Gauge | MetricType::Counter)
            })
            .flat_map(|(name, family)| {
                iter::zip(iter::repeat((name, family)), family.metrics.iter())
            })
            .for_each(|((family_name, family), (&uuid, metric))| {
                self.do_update_metric(
                    uuid,
                    metric,
                    family,
                    family_name,
                    do_update_minute,
                    do_update_hour,
                    do_update_day,
                )
            });
    }

    /// Update the metric in the entry database.
    fn do_update_metric(
        &mut self,
        uuid: uuid::Uuid,
        metric: &Metric,
        family: &MetricFamily,
        family_name: &str,
        do_update_minute: bool,
        do_update_hour: bool,
        do_update_day: bool,
    ) {
        // Get (or create) the entry.
        let entry = self.entry_database.entry(uuid).or_insert_with(|| {
            tracing::debug!("New entry {uuid}");
            let (v2_name, metadata) = Self::to_name_v2(&self.mappings, metric, family, family_name)
                .expect("Unexpected to_name_v2 failure");
            let (owner_part, owner) = get_owner_part(metric, self.host_uuid);

            // Consider only AVERAGE metrics for now.
            RrdEntry::new(
                format!("AVERAGE:{owner_part}:{v2_name}").into_boxed_str(),
                owner,
                metadata,
            )
        });

        // Take the first metric.
        let first_metric = metric
            .metrics_point
            .first()
            .map(|metric_point| &metric_point.value);

        // Get the value as f64, use NaN is nothing is available.
        let value = first_metric.map_or(f64::NAN, |metric| match metric {
            MetricValue::Gauge(value) | MetricValue::Counter { total: value, .. } => match *value {
                NumberValue::Double(val) => val,
                NumberValue::Int64(val) => val as _,
                NumberValue::Undefined => f64::NAN,
            },
            _ => f64::NAN,
        });

        entry.five_seconds.push(value);

        if do_update_minute {
            self.latest_minute_update = SystemTime::now();
            entry.minute.push(value);
        }

        if do_update_hour {
            self.latest_hour_update = SystemTime::now();
            entry.hour.push(value);
        }

        if do_update_day {
            self.latest_day_update = SystemTime::now();
            entry.day.push(value);
        }
    }

    pub async fn process_message(&self, message: RrddServerMessage) {
        match message {
            RrddServerMessage::RequestRrdUpdates(info, sender) => {
                tracing::info!("Processing RrdUpdate request");

                // TODO: Use interval.

                let granuality = {
                    let distance_from_now = info.start.elapsed().unwrap_or(
                        Duration::ZERO, /* if start is in the future, consider now */
                    );

                    if distance_from_now < Granuality::FiveSeconds.get_covered_duration() {
                        Granuality::FiveSeconds
                    } else if distance_from_now < Granuality::Minute.get_covered_duration() {
                        Granuality::Minute
                    } else if distance_from_now < Granuality::Hour.get_covered_duration() {
                        Granuality::Hour
                    } else {
                        Granuality::Day
                    }
                };

                let (legend, mut data_iterators): (Vec<_>, Vec<_>) = self
                    .entry_database
                    .values()
                    .filter(|entry| {
                        // Apply filter
                        match info.filter {
                            RrdXportFilter::All => true,
                            RrdXportFilter::AllNoHost => !matches!(entry.owner, DataSourceOwner::Host),
                            RrdXportFilter::VM(uuid) => matches!(entry.owner, DataSourceOwner::VM(entry_uuid) if uuid == entry_uuid),
                            RrdXportFilter::SR(uuid) => matches!(entry.owner, DataSourceOwner::SR(entry_uuid) if uuid == entry_uuid),
                        }
                    })
                    .map(|entry| (entry.name.clone(), entry.get_buffer(granuality).iter()))
                    .unzip();

                let (start, end) = {
                    let start = match granuality {
                        Granuality::FiveSeconds => self.latest_update,
                        Granuality::Minute => self.latest_minute_update,
                        Granuality::Hour => self.latest_hour_update,
                        Granuality::Day => self.latest_day_update,
                    };

                    (start, start + granuality.get_covered_duration())
                };

                let data = (0..granuality.get_buffer_size())
                    .map(|i| {
                        (
                            start + (i as u32) * granuality.get_interval(),
                            data_iterators
                                .iter_mut()
                                .map(|iter| iter.next().unwrap_or(&f64::NAN))
                                .cloned()
                                .collect(),
                        )
                    })
                    .collect();

                sender
                    .send(Ok(RrdXport {
                        start,
                        end,
                        step_secs: granuality.get_five_seconds_interval() * 5,
                        legend,
                        data,
                    }))
                    .await
                    .unwrap();
            }
        }
    }

    #[tracing::instrument]
    pub fn start(mut self, hub_channel: mpsc::UnboundedSender<HubPushMessage>) -> JoinHandle<()> {
        tokio::task::spawn(async move {
            let mut timer = tokio::time::interval(Duration::from_secs(5));

            loop {
                select! {
                    _ = timer.tick() => {
                        tracing::debug!("Pulling metrics");

                        match self.pull_metrics(&hub_channel).await {
                            Ok(metrics) => self.update_metrics(&metrics),
                            Err(e) => tracing::error!("Unable to pull metrics {e}")
                        }
                    },
                    msg = self.receiver.recv() => {
                        match msg {
                            Some(msg) => self.process_message(msg).await,
                            None => tracing::error!("Unable to read channel message")
                        }
                    }
                }
            }
        })
    }
}
