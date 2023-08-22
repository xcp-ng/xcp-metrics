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
use futures::Stream;
use tokio::sync::{mpsc, Mutex};

use super::xs::{XBTransaction, XsStreamTrait, XsTrait, XsWatchEntry};

pub struct MockXs {
    tree: DashMap<Box<str>, Box<str>>,

    watch_map: DashSet<(Box<str>, Box<str>)>,
    watch_reader: Mutex<mpsc::UnboundedReceiver<XsWatchEntry>>,
    watch_sender: mpsc::UnboundedSender<XsWatchEntry>,
}

impl Default for MockXs {
    fn default() -> Self {
        Self::new(DashMap::default())
    }
}

impl MockXs {
    pub fn new(tree: DashMap<Box<str>, Box<str>>) -> Self {
        let (sender, reader) = mpsc::unbounded_channel();

        Self {
            tree,
            watch_map: Default::default(),
            watch_reader: Mutex::new(reader),
            watch_sender: sender,
        }
    }
}

pub struct MockStream<'a> {
    fake_xs: &'a MockXs,
}

impl Stream for MockStream<'_> {
    type Item = XsWatchEntry;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let Ok(mut reader) = self.fake_xs.watch_reader.try_lock() else {
            return Poll::Ready(None)
        };

        reader.poll_recv(cx)
    }
}

impl<'a> XsStreamTrait<'a> for MockStream<'a> {}

impl XsTrait for MockXs {
    type XsStreamType<'a> = MockStream<'a>
  where
      Self: 'a;

    fn directory(&self, _: XBTransaction, path: &str) -> Result<Vec<String>, Error> {
        if self.tree.get(path).is_some() {
            let entries: Vec<String> = self
                .tree
                .iter()
                .filter_map(|entry| {
                    let key = entry.key().as_ref();
                    let prefix = format!("{path}/");

                    // This is at least a entry (or sub-entry) of this directory.
                    if key.starts_with(&prefix) {
                        // Strip the 'path/' part.
                        let without_prefix: String = key.chars().skip(prefix.len()).collect();

                        // If there is a '/' with path with prefix, this is a subdirectory entry, skip it.
                        if without_prefix.contains('/') {
                            None
                        } else {
                            Some(without_prefix)
                        }
                    } else {
                        None
                    }
                })
                .collect();

            Ok(entries)
        } else {
            Err(Error::new(ErrorKind::NotFound, "Not found"))
        }
    }

    fn read(&self, _: XBTransaction, path: &str) -> Result<String, Error> {
        self.tree
            .get(path)
            .map(|entry| entry.value().to_string())
            .ok_or(Error::new(ErrorKind::NotFound, "Not found"))
    }

    fn write(&self, _: XBTransaction, path: &str, data: &str) -> Result<(), Error> {
        self.tree.insert(path.into(), data.into());

        // Create all parents (if needed)
        // Take all path parts (subdirectory names), and build the path progressively,
        // creating all missings entries in the tree.
        path.split('/').fold(String::from("/"), |mut path, part| {
            path.push_str(part);

            // Create the directory (if not exists)
            if self.tree.get(path.as_str()).is_none() {
                self.tree.insert(path.clone().into(), "".into());
            }

            path
        });

        // Fire any related watcher
        for entry in self.watch_map.iter() {
            // Make sure it is either the same path or a subdirectory and not
            // a path in the same directory, with a name that has the same prefix.
            if entry.0.as_ref() == path || path.starts_with(&format!("{}/", entry.0)) {
                self.watch_sender
                    .send(XsWatchEntry {
                        path: path.into(),
                        token: entry.1.to_string(),
                    })
                    .map_err(|_| Error::new(ErrorKind::BrokenPipe, "Unable to send entry"))?;
            }
        }

        Ok(())
    }

    fn rm(&self, _: XBTransaction, path: &str) -> Result<(), Error> {
        self.tree.remove(path);

        Ok(())
    }

    fn watch(&self, path: &str, token: &str) -> Result<(), Error> {
        self.watch_map.insert((path.into(), token.into()));

        Ok(())
    }

    fn read_watch(&self) -> Result<Vec<XsWatchEntry>, Error> {
        let mut watcher = self.watch_reader.blocking_lock();

        Ok(iter::from_fn(|| watcher.try_recv().ok()).collect())
    }

    fn check_watch(&self) -> Result<Option<XsWatchEntry>, Error> {
        let mut watcher = self.watch_reader.blocking_lock();

        Ok(watcher.try_recv().ok())
    }

    fn unwatch(&self, path: &str, token: &str) -> Result<(), Error> {
        self.watch_map
            .retain(|entry| !(entry.0.as_ref() == path && entry.1.as_ref() == token));

        Ok(())
    }

    fn get_stream<'a>(&'a self) -> Result<Self::XsStreamType<'a>, Error> {
        Ok(MockStream { fake_xs: self })
    }
}

#[test]
fn test_watch() {
    let xs = MockXs::default();

    xs.watch("/test1", "token").unwrap();
    xs.watch("/test2", "token").unwrap();

    xs.write(XBTransaction::Null, "/test1", "test").unwrap();
    xs.write(XBTransaction::Null, "/test2", "test").unwrap();
    xs.write(XBTransaction::Null, "/test2/123", "test").unwrap();

    let watched = xs.read_watch().unwrap();
    assert!(watched[0].path == "/test1");
    assert!(watched[1].path == "/test2");
    assert!(watched[2].path == "/test2/123");
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
