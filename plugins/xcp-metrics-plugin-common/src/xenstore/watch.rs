use futures::{Stream, StreamExt};
use std::io::Error as IoError;

use super::read::XsRead;

#[derive(Debug)]
pub struct XsWatchEntry {
    pub path: String,
    pub token: String,
}

pub trait XsStreamTrait<'a>: Send + Sync + Stream<Item = XsWatchEntry> + Unpin {}

pub trait XsWatch: XsRead + 'static + Send + Sync {
    type XsStreamType<'a>: XsStreamTrait<'a>
    where
        Self: 'a;

    fn watch(&self, path: &str, token: &str) -> Result<(), IoError>;

    fn read_watch(&self) -> Result<Vec<XsWatchEntry>, IoError>;

    fn check_watch(&self) -> Result<Option<XsWatchEntry>, IoError>;

    fn unwatch(&self, path: &str, token: &str) -> Result<(), IoError>;

    fn get_stream(&self) -> Result<Self::XsStreamType<'_>, IoError>;
}

#[cfg(feature = "xenstore-wip")]
impl From<super::xs::XsWatchEntry> for XsWatchEntry {
    fn from(super::xs::XsWatchEntry { path, token }: super::xs::XsWatchEntry) -> Self {
        Self { path, token }
    }
}

#[cfg(feature = "xenstore-wip")]
pub struct XsStream<'a>(super::xs::XsStream<'a>);

#[cfg(feature = "xenstore-wip")]
impl Stream for XsStream<'_> {
    type Item = XsWatchEntry;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.0
            .poll_next_unpin(cx)
            .map(|poll_result| poll_result.map(Into::into))
    }
}

#[cfg(feature = "xenstore-wip")]
impl XsStreamTrait<'_> for XsStream<'_> {}

#[cfg(feature = "xenstore-wip")]
impl XsWatch for super::xs::Xs {
    type XsStreamType<'a> = XsStream<'a>;

    fn watch(&self, path: &str, token: &str) -> Result<(), IoError> {
        self.watch(path, token)
    }

    fn read_watch(&self) -> Result<Vec<XsWatchEntry>, IoError> {
        self.read_watch()
            .map(|entries| entries.into_iter().map(Into::into).collect())
    }

    fn check_watch(&self) -> Result<Option<XsWatchEntry>, IoError> {
        self.check_watch().map(|result| result.map(Into::into))
    }

    fn unwatch(&self, path: &str, token: &str) -> Result<(), IoError> {
        self.unwatch(path, token)
    }

    fn get_stream(&self) -> Result<Self::XsStreamType<'_>, IoError> {
        Ok(XsStream(self.get_stream()?))
    }
}
