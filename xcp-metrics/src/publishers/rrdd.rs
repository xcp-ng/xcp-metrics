use std::sync::Arc;

use tokio::sync::mpsc;
use xcp_metrics_common::rrdd::protocol_v2::{RrddMessageHeader, RrddMetadata};

pub struct RrddMetricUnit {
    source_uid: Box<str>,
    header: RrddMessageHeader,
    metadata: Arc<RrddMetadata>,
}

pub enum RrddIPCMessage {
    AddPlugin(Box<str>),
}

pub struct RrddServer;

impl RrddServer {
    pub async fn run() -> anyhow::Result<Self> {
        let (metrics_sender, metrics_receiver) = mpsc::unbounded_channel();

        Self::start_rpc_server(metrics_sender).await?;

        Ok(Self)
    }
}
