use std::{collections::HashMap, fmt::Write, iter, sync::Arc, time::Duration};
use tokio::{self, select, sync::mpsc, task::JoinHandle};

use xcp_metrics_common::{
    metrics::{Metric, MetricSet, MetricType, MetricValue, NumberValue},
    rrdd::rrd_updates::{RrdXport, RrdXportInfo},
};

use super::{RrdEntry, DAY_UPDATES_INTERVAL, HOUR_UPDATES_INTERVAL, MINUTE_UPDATES_INTERVAL};

use crate::hub::{HubPullResponse, HubPushMessage, PullMetrics};

#[derive(Debug)]
pub enum RrddServerMessage {
    RequestRrdUpdates(RrdXportInfo, mpsc::Sender<anyhow::Result<RrdXport>>),
}

#[derive(Debug)]
pub struct RrddServer {
    pub receiver: mpsc::UnboundedReceiver<RrddServerMessage>,
    pub host_uuid: uuid::Uuid,

    /// Map a UUID from hub's MetricSet with a RrdEntry.
    pub entry_database: HashMap<uuid::Uuid, RrdEntry>,

    pub minute_update_counter: u32,
    pub hour_update_counter: u32,
    pub day_update_counter: u32,
}

enum Granuality {
    FiveSeconds,
    Minute,
    Hour,
    Day,
}

fn to_name_v2(metric: &Metric, family_name: &str) -> String {
    metric
        .labels
        .iter()
        // Ignore owner label
        .filter(|l| l.0.as_ref() != "owner")
        .fold(String::from(family_name), |mut buffer, label| {
            write!(buffer, "_{}", label.1).ok();
            buffer
        })
}

/// Get the owner part of the rrd name (i.e vm:UUID).
fn get_owner_part(metric: &Metric, host_uuid: uuid::Uuid) -> String {
    metric
        .labels
        .iter()
        .filter(|l| l.0.as_ref() == "owner")
        .find_map(|l| {
            let parts: Vec<&str> = l.1.split_ascii_whitespace().take(2).collect();
            let (kind, uuid) = (parts.first()?, parts.get(1)?);

            Some(format!("{kind}:{uuid}"))
        })
        .unwrap_or_else(|| format!("host:{}", host_uuid.as_hyphenated()))
}

impl RrddServer {
    pub fn new() -> (Self, mpsc::UnboundedSender<RrddServerMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();

        (
            Self {
                receiver,
                host_uuid: uuid::Uuid::new_v4(),

                entry_database: HashMap::new(),

                day_update_counter: 0,
                hour_update_counter: 0,
                minute_update_counter: 0,
            },
            sender,
        )
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
            self.minute_update_counter = (self.minute_update_counter + 1) % MINUTE_UPDATES_INTERVAL;
            self.minute_update_counter == 0
        };

        let do_update_hour = {
            self.hour_update_counter = (self.hour_update_counter + 1) % HOUR_UPDATES_INTERVAL;
            self.hour_update_counter == 0
        };

        let do_update_day = {
            self.day_update_counter = (self.day_update_counter + 1) % DAY_UPDATES_INTERVAL;
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
            .for_each(|((family_name, _), (&uuid, metric))| {
                self.do_update_metric(
                    uuid,
                    metric,
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
        family_name: &str,
        do_update_minute: bool,
        do_update_hour: bool,
        do_update_day: bool,
    ) {
        // Get (or create) the entry.
        let entry = self.entry_database.entry(uuid).or_insert_with(|| {
            let v2_name = to_name_v2(metric, family_name);
            let owner_part = get_owner_part(metric, self.host_uuid);

            // Consider only AVERAGE metrics for now.
            RrdEntry::new(format!("AVERAGE:{owner_part}:{v2_name}").into_boxed_str())
        });

        // Take the first metric.
        let first_metric = metric
            .metrics_point
            .get(0)
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
            entry.minute.push(value);
        }

        if do_update_hour {
            entry.hour.push(value);
        }

        if do_update_day {
            entry.minute.push(value);
        }
    }

    pub async fn process_message(&self, message: RrddServerMessage) {
        match message {
            RrddServerMessage::RequestRrdUpdates(info, sender) => {
                let (legend, mut data_iterators): (_, Vec<_>) = self
                    .entry_database
                    .values()
                    .map(|entry| (entry.name.clone(), entry.five_seconds.iter()))
                    .unzip();

                let granuality = Granuality::FiveSeconds;

                let data = (0..RrdEntry::FIVE_SECONDS_BUFFER_SIZE)
                    .map(|i| {
                        (
                            info.start + Duration::from_secs(i as _),
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
                        start: info.start,
                        end: info.end,
                        step_secs: info.step_secs,
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
