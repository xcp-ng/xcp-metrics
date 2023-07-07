//! Metrics providers.
use tokio::{sync::mpsc, task::JoinHandle};

use crate::hub::HubPushMessage;

pub mod protocol_v2;

pub trait Provider {
    fn start_provider(self, hub_channel: mpsc::UnboundedSender<HubPushMessage>) -> JoinHandle<()>;
}
