use xcp_metrics_common::xapi::hyper::{Body, Request, Response};

pub async fn rpc_entrypoint(req: Request<Body>) -> anyhow::Result<Response<Body>> {
    println!("{req:#?}");

    Ok(Response::new("Hello, World".into()))
}
