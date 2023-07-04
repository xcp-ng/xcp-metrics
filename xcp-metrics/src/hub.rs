use std::sync::Arc;

use tokio::{
    sync::mpsc,
    task::{self, JoinHandle},
};
use xcp_metrics_common::metrics::{Metric, MetricPoint, MetricSet};

#[derive(Debug, Clone)]
pub struct RegisterMetrics {
    pub family: Box<str>,
    pub metrics: Metric,
    pub uuid: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct UnregisterMetrics {
    pub uuid: uuid::Uuid,
}

#[derive(Debug, Clone)]
pub struct UpdateMetrics {
    pub uuid: uuid::Uuid,
    pub new_values: Box<[MetricPoint]>,
}

#[derive(Debug, Clone)]
pub struct PullMetrics(pub mpsc::UnboundedSender<HubPullResponse>);

#[derive(Debug, Clone)]
pub enum HubPushMessage {
    RegisterMetrics(RegisterMetrics),
    UnregisterMetrics(UnregisterMetrics),
    UpdateMetrics(UpdateMetrics),
    PullMetrics(PullMetrics),
}

#[derive(Debug, Clone)]
pub enum HubPullResponse {
    Metrics(Arc<MetricSet>),
}

#[derive(Debug, Clone, Default)]
pub struct MetricsHub {
    metrics: Arc<MetricSet>,
}

impl MetricsHub {
    pub async fn start(self) -> (JoinHandle<()>, mpsc::UnboundedSender<HubPushMessage>) {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut rendez_vous = mpsc::channel(1);

        let handle = task::spawn(async move { self.run(receiver, rendez_vous.0).await });

        rendez_vous.1.recv().await;

        (handle, sender)
    }

    async fn run(
        mut self,
        mut receiver: mpsc::UnboundedReceiver<HubPushMessage>,
        rendez_vous: mpsc::Sender<()>,
    ) {
        rendez_vous.send(()).await.unwrap();

        while let Some(msg) = receiver.recv().await {
            println!("Hub: {msg:?}");
            match msg {
                HubPushMessage::RegisterMetrics(message) => self.register(message).await,
                HubPushMessage::UnregisterMetrics(message) => self.unregister(message).await,
                HubPushMessage::UpdateMetrics(message) => self.update(message).await,
                HubPushMessage::PullMetrics(message) => self.pull_metrics(message).await,
            }

            println!("Hub: Metrics status:");
            //println!("{:#?}", self.metrics);
        }

        println!("Stopped hub")
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
        let mut deprecated_family = None;

        for (family_name, family) in metrics.families.iter_mut() {
            if family.metrics.remove(&message.uuid).is_some() {
                println!("Unregistered {}", message.uuid);

                // Remove metric family if now empty.
                if family.metrics.is_empty() {
                    deprecated_family.replace(family_name.clone());
                }

                break;
            }

        }

        if let Some(name) = &deprecated_family {
            println!("Unregistered empty metric family {name}");
            metrics.families.remove(name);
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
