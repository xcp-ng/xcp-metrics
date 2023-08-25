//! A fake XenStore.
//!
//! Doesn't really attempt to be exactly compatible with XenStore implementation.
use std::{
    io::{Error, ErrorKind},
    iter,
    pin::Pin,
    task::{Context, Poll},
};

use dashmap::{DashMap, DashSet};
use tokio::sync::{mpsc, Mutex};

use super::xs::{XBTransaction, XsTrait};

pub struct MockXs {
    tree: DashMap<Box<str>, Box<str>>,
}

impl Default for MockXs {
    fn default() -> Self {
        Self::new(DashMap::default())
    }
}

impl MockXs {
    pub fn new(tree: DashMap<Box<str>, Box<str>>) -> Self {
        Self {
            tree,
        }
    }
}


#[test]
fn test_subdirectories() {
    let xs = MockXs::default();

    xs.write(XBTransaction::Null, "/1/2/3/4/5", "hello world")
        .unwrap();

    for path in ["/1", "/1/2", "/1/2/3", "/1/2/3/4", "/1/2/3/4/5"] {
        xs.read(XBTransaction::Null, path)
            .expect(&format!("Missing {path}"));
    }
}
