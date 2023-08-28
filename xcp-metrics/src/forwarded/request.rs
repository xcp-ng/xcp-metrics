use std::{collections::HashMap, str::FromStr};

use serde::Deserialize;
use xapi::hyper::{
    header::{ACCEPT, CONTENT_LENGTH, CONTENT_TYPE, HOST, TRANSFER_ENCODING, USER_AGENT},
    http::uri::PathAndQuery,
    Body, Request, Version,
};

/// xapi-project/xen-api/blob/master/ocaml/libs/http-lib/http.ml for reference
#[derive(Clone, Debug, Deserialize)]
pub struct ForwardedRequest {
    pub m: Box<str>,
    pub uri: Box<str>,
    pub query: HashMap<Box<str>, Box<str>>,
    pub version: Box<str>,
    pub frame: bool,
    pub transfer_encoding: Option<Box<str>>,
    pub accept: Option<Box<str>>,
    pub content_length: Option<usize>,
    pub auth: Option<Box<[Box<str>]>>,
    pub cookie: HashMap<Box<str>, Box<str>>,
    pub task: Option<Box<str>>,
    pub subtask_of: Option<Box<str>>,
    pub content_type: Option<Box<str>>,
    pub host: Option<Box<str>>,
    pub user_agent: Option<Box<str>>,
    pub close: bool,
    pub additional_headers: HashMap<Box<str>, Box<str>>,
    pub body: Option<Box<str>>,
    pub traceparent: Option<Box<str>>,
}

impl TryFrom<ForwardedRequest> for Request<Body> {
    type Error = anyhow::Error;

    fn try_from(request: ForwardedRequest) -> Result<Self, Self::Error> {
        let mut builder = Request::builder();

        if let Some(version) = match request.version.as_ref() {
            "HTTP/0.9" => Some(Version::HTTP_09),
            "HTTP/1.0" => Some(Version::HTTP_10),
            "HTTP/1.1" => Some(Version::HTTP_11),
            "HTTP/2.0" => Some(Version::HTTP_2),
            "HTTP/3.0" => Some(Version::HTTP_3),
            _ => None,
        } {
            builder = builder.version(version);
        }

        builder = builder.method(request.m.as_ref());

        builder = builder.uri(PathAndQuery::from_str(&request.uri)?);

        if let Some(value) = request.content_length {
            builder = builder.header(CONTENT_LENGTH, value);
        }

        if let Some(transfer_encoding) = request.transfer_encoding {
            builder = builder.header(TRANSFER_ENCODING, transfer_encoding.as_ref());
        }

        if let Some(accept) = request.accept {
            builder = builder.header(ACCEPT, accept.as_ref());
        }

        if let Some(content_type) = request.content_type {
            builder = builder.header(CONTENT_TYPE, content_type.as_ref());
        }

        if let Some(host) = request.host {
            builder = builder.header(HOST, host.as_ref());
        }

        if let Some(user_agent) = request.user_agent {
            builder = builder.header(USER_AGENT, user_agent.as_ref());
        }

        for (name, value) in request.additional_headers.iter() {
            builder = builder.header(name.as_ref(), value.as_ref());
        }

        Ok(builder.body(match request.body {
            Some(content) => Body::from(content.as_bytes().to_vec()),
            None => Body::empty(),
        })?)
    }
}
