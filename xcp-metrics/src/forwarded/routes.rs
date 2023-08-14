use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};

use tokio::sync::mpsc;

use xcp_metrics_common::xapi::hyper::{Body, Response};

use crate::{
    publishers::rrdd::{server::RrddServerMessage, RrdXportFilter, RrdXportInfo},
    rpc, XcpMetricsShared,
};

use super::request::ForwardedRequest;

pub(super) async fn route_forwarded(
    shared: Arc<XcpMetricsShared>,
    request: ForwardedRequest,
) -> anyhow::Result<Response<Body>> {
    match request.uri.as_ref() {
        "/rrd_updates" => rrd_update_handler(shared, request).await,
        "/" => rpc::entrypoint(shared, request.try_into()?).await,
        _ => Response::builder()
            .status(404)
            .body("Invalid request".into())
            .map_err(|err| anyhow::anyhow!(err)),
    }
}

async fn rrd_update_handler(
    shared: Arc<XcpMetricsShared>,
    request: ForwardedRequest,
) -> anyhow::Result<Response<Body>> {
    let (tx, mut rx) = mpsc::channel(1);

    let with_host = request
        .query
        .get("host")
        .map(|v| v.as_ref() == "true")
        .unwrap_or(false);

    let use_json = request
        .query
        .get("json")
        .map(|v| v.as_ref() == "true")
        .unwrap_or(false);

    let filter = if with_host {
        RrdXportFilter::All
    } else {
        RrdXportFilter::AllNoHost
    };

    let start = if let Some(value) = request.query.get("start") {
        let since_epoch = value.parse()?;

        SystemTime::UNIX_EPOCH + Duration::from_secs(since_epoch)
    } else {
        SystemTime::now()
    };

    let interval = if let Some(value) = request.query.get("interval") {
        value.parse()?
    } else {
        1
    };

    shared
        .rrdd_channel
        .send(RrddServerMessage::RequestRrdUpdates(
            RrdXportInfo {
                start,
                interval,
                filter,
            },
            tx,
        ))?;

    let response = rx
        .recv()
        .await
        .ok_or(anyhow::anyhow!("No value received from channel"))??;

    let buffer = if use_json {
        let mut bytes = Vec::with_capacity(1024);
        response.write_json(&mut bytes)?;
        bytes
    } else {
        let mut s = String::new();
        response.write_xml(&mut s)?;
        s.into_bytes()
    };

    Response::builder()
        .status(200)
        .body(Body::from(buffer))
        .map_err(|err| anyhow::anyhow!(err))
}
