//! RPC routes
mod deregister;
mod get_formats;
mod next_reading;
mod register;
mod register_v3;

use std::sync::Arc;

use http::Response;
use hyper::body::Bytes;
use xapi::rpc::message::{error::RpcError, request::RpcRequest};

use crate::{publishers::openmetrics, XcpMetricsShared};

fn make_binary(response: Response<String>) -> Response<Bytes> {
    // Convert inner String to Bytes.
    response.map(Into::into)
}

pub async fn dispatch(shared: Arc<XcpMetricsShared>, request: RpcRequest) -> Response<Bytes> {
    tracing::info!("RPC Message: {request}");

    let res = match request.get_name() {
        "OpenMetrics" => openmetrics::run(shared, request).await,
        "Plugin.Local.register" => register::run(shared, request).await.map(make_binary),
        "Plugin.Local.deregister" => deregister::run(shared, request).await.map(make_binary),
        "Plugin.Local.next_reading" => next_reading::run(shared, request).await.map(make_binary),
        "Plugin.Metrics.get_versions" => get_formats::run(shared, request).await.map(make_binary),
        "Plugin.Metrics.register" => register_v3::run(shared, request).await.map(make_binary),
        "Plugin.Metrics.deregister" => deregister::run(shared, request).await.map(make_binary),
        _ => {
            tracing::error!("RPC Method not found: {request}");
            RpcError::respond_to::<()>(Some(&request), -32601, "Method not found", None)
                .map(make_binary)
        }
    };

    match res {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("Internal error: {e}");

            Response::builder()
                .status(http::StatusCode::INTERNAL_SERVER_ERROR)
                .body("Internal Server Error".into())
                .expect("Unable to make error 500")
        }
    }
}
