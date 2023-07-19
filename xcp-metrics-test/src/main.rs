use std::{
    collections::HashMap,
    io::{Read, Write},
    os::unix::net::UnixStream,
};

use serde::Serialize;

#[derive(Clone, Debug, Default, Serialize)]
struct ForwardedRequest {
    pub m: String,
    pub uri: String,
    pub query: HashMap<String, String>,
    pub version: String,
    pub frame: bool,
    pub transfer_encoding: Option<String>,
    pub accept: Option<String>,
    pub content_length: Option<usize>,
    pub auth: Option<Vec<String>>,
    pub cookie: HashMap<String, String>,
    pub task: Option<String>,
    pub subtask_of: Option<String>,
    pub content_type: Option<String>,
    pub host: Option<String>,
    pub user_agent: Option<String>,
    pub close: bool,
    pub additional_headers: HashMap<String, String>,
    pub body: Option<String>,
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

    socket
        .write_all(serde_json::to_string(&request).unwrap().as_bytes())
        .unwrap();

    let mut buffer = String::new();
    socket.read_to_string(&mut buffer).unwrap();
    println!("{buffer}");
}
