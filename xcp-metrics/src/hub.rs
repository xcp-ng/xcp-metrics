use std::sync::Arc;

use tokio::sync::mpsc;
use xcp_metrics_common::metrics::{Metric, MetricPoint, MetricSet};

#[derive(Clone)]
pub struct RegisterMetrics {
    family: Box<str>,
    metrics: Metric,
    uuid: uuid::Uuid,
}

#[derive(Clone)]
pub struct UnregisterMetrics {
    uuid: uuid::Uuid,
}

#[derive(Clone)]
pub struct UpdateMetrics {
    uuid: uuid::Uuid,
    new_values: Box<[MetricPoint]>,
}

#[derive(Clone)]
pub struct PullMetrics(pub mpsc::UnboundedSender<HubPullResponse>);

#[derive(Clone)]
pub enum HubPushMessage {
    CreateFamily(/* ? */),
    RegisterMetrics(RegisterMetrics),
    UnregisterMetrics(UnregisterMetrics),
    UpdateMetrics(UpdateMetrics),
    PullMetrics(PullMetrics),
}

#[derive(Clone)]
pub enum HubPullResponse {
    Metrics(Arc<MetricSet>),
}

#[derive(Clone, Default)]
pub struct MetricsHub {
    metrics: Arc<MetricSet>,
}

impl MetricsHub {
    pub async fn start(self) -> mpsc::UnboundedSender<HubPushMessage> {
        let (sender, receiver) = mpsc::unbounded_channel();

        tokio::task::spawn(async move { self.run(receiver).await });

        sender
    }

    async fn run(mut self, mut receiver: mpsc::UnboundedReceiver<HubPushMessage>) {
        while let Some(msg) = receiver.recv().await {
            match msg {
                HubPushMessage::CreateFamily() => todo!(),
                HubPushMessage::RegisterMetrics(message) => self.register(message).await,
                HubPushMessage::UnregisterMetrics(message) => self.unregister(message).await,
                HubPushMessage::UpdateMetrics(message) => self.update(message).await,
                HubPushMessage::PullMetrics(message) => self.pull_metrics(message).await,
            }
        }
    }

    async fn register(&mut self, message: RegisterMetrics) {
        let metrics = Arc::make_mut(&mut self.metrics);

        let family = match metrics.families.get_mut(&message.family) {
            Some(f) => f,
            None => {
                metrics
                    .families
                    .insert(message.family.clone(), Default::default());
                metrics.families.get_mut(&message.family).unwrap()
            }
        };

        if let Some(old) = family.metrics.insert(message.uuid, message.metrics) {
            eprintln!("Overriden {old:?}");
        }
    }

    async fn unregister(&mut self, message: UnregisterMetrics) {
        let metrics = Arc::make_mut(&mut self.metrics);

        for (_, family) in metrics.families.iter_mut() {
            if let Some(_) = family.metrics.remove(&message.uuid) {
                println!("Unregistered {}", message.uuid);
                break;
            }
        }
    }

    async fn update(&mut self, mut message: UpdateMetrics) {
        let metrics = Arc::make_mut(&mut self.metrics);

        // TODO: Do some checks.

        for (_, family) in metrics.families.iter_mut() {
            if let Some(metrics) = family.metrics.get_mut(&message.uuid) {
                /* Rust wizardry */
                std::mem::swap(&mut metrics.metrics_point, &mut message.new_values);
                break;
            }
        }
    }

    async fn pull_metrics(&mut self, message: PullMetrics) {
        let sender = message.0;

        if let Err(e) = sender.send(HubPullResponse::Metrics(Arc::clone(&self.metrics))) {
            eprintln!("Error occured in a channel {e}");
        }
    }
}
