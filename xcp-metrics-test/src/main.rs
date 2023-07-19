use std::{
    collections::HashMap,
    io::{Read, Write},
    os::unix::net::UnixStream,
};

use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
#[serde()]
struct ForwardedRequest {
    pub m: String,
    pub uri: String,
    pub query: HashMap<String, String>,
    pub version: String,
    pub frame: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_length: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Vec<String>>,
    pub cookie: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtask_of: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    pub close: bool,
    pub additional_headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub traceparent: Option<String>,
}

fn main() {
    let path = xcp_metrics_common::xapi::get_module_path("xcp-rrdd.forwarded");
    let mut socket = UnixStream::connect(path).unwrap();

    let request = ForwardedRequest {
        m: "Get".into(),
        uri: "/rrd_updates".into(),
        query: HashMap::default(),
        version: "1.1".into(),
        ..Default::default()
    };

    let request_json = serde_json::to_string(&request).unwrap();
    println!("Sending {request_json}");

    socket
        .write_all(request_json.as_bytes())
        .unwrap();

    let mut buffer = String::new();
    let result = socket.read_to_string(&mut buffer);
    println!("{result:?} {buffer:?}");
}
