use std::{sync::Arc, time::SystemTime};

use tokio::sync::mpsc;
use xcp_metrics_common::{rrdd::rrd_updates::RrdXportInfo, xapi::hyper::Response};

use super::ForwardedRequest;
use crate::{publishers::rrdd::server::RrddServerMessage, XcpMetricsShared};

pub(super) async fn route_forwarded(
    shared: Arc<XcpMetricsShared>,
    request: ForwardedRequest,
) -> anyhow::Result<Response<Vec<u8>>> {
    match request.uri.as_ref() {
        "/rrd_updates" => rrd_update_handler(shared, request).await,
        _ => Response::builder()
            .status(404)
            .body("Invalid request".to_string().as_bytes().to_vec())
            .map_err(|err| anyhow::anyhow!(err)),
    }
}

async fn rrd_update_handler(
    shared: Arc<XcpMetricsShared>,
    request: ForwardedRequest,
) -> anyhow::Result<Response<Vec<u8>>> {
    let (tx, mut rx) = mpsc::channel(1);

    shared
        .rrdd_channel
        .send(RrddServerMessage::RequestRrdUpdates(
            RrdXportInfo {
                start: SystemTime::now(),
                end: SystemTime::now(),
                step_secs: 5,
            },
            tx,
        ))?;

    let response = rx
        .recv()
        .await
        .ok_or(anyhow::anyhow!("No value received from channel"))??;

    let mut buffer = String::new();
    response.write_xml(&mut buffer)?;

    Response::builder()
        .status(200)
        .body(buffer.as_bytes().to_vec())
        .map_err(|err| anyhow::anyhow!(err))
}
