use std::error::Error;
use std::io::Error as IoError;

use futures::Stream;
use xenstore_rs::XsStream;
pub use xenstore_rs::{XBTransaction, Xs, XsOpenFlags, XsWatchEntry};

pub trait XsTrait: 'static + Sized + Send + Sync {
    type XsStreamType<'a>: XsStreamTrait<'a>
    where
        Self: 'a;

    fn new(open_type: XsOpenFlags) -> Result<Self, Box<dyn Error>>;

    fn directory(&self, transaction: XBTransaction, path: &str) -> Result<Vec<String>, IoError>;

    fn read(&self, transaction: XBTransaction, path: &str) -> Result<String, IoError>;

    fn write(&self, transaction: XBTransaction, path: &str, data: &str) -> Result<(), IoError>;

    fn rm(&self, transaction: XBTransaction, path: &str) -> Result<(), IoError>;

    fn watch(&self, path: &str, token: &str) -> Result<(), IoError>;

    fn read_watch(&self) -> Result<Vec<XsWatchEntry>, IoError>;

    fn check_watch(&self) -> Result<Option<XsWatchEntry>, IoError>;

    fn unwatch(&self, path: &str, token: &str) -> Result<(), IoError>;

    fn get_stream(&self) -> Result<Self::XsStreamType<'_>, IoError>;
}

pub trait XsStreamTrait<'a>: Send + Sync + Stream<Item = XsWatchEntry> + Unpin {}

impl XsTrait for Xs {
    type XsStreamType<'a> = XsStream<'a>;

    fn new(open_type: XsOpenFlags) -> Result<Self, Box<dyn Error>> {
        Self::new(open_type)
    }

    fn directory(&self, transaction: XBTransaction, path: &str) -> Result<Vec<String>, IoError> {
        self.directory(transaction, path)
    }

    fn read(&self, transaction: XBTransaction, path: &str) -> Result<String, IoError> {
        self.read(transaction, path)
    }

    fn write(&self, transaction: XBTransaction, path: &str, data: &str) -> Result<(), IoError> {
        self.write(transaction, path, data)
    }

    fn rm(&self, transaction: XBTransaction, path: &str) -> Result<(), IoError> {
        self.rm(transaction, path)
    }

    fn watch(&self, path: &str, token: &str) -> Result<(), IoError> {
        self.watch(path, token)
    }

    fn read_watch(&self) -> Result<Vec<XsWatchEntry>, IoError> {
        self.read_watch()
    }

    fn check_watch(&self) -> Result<Option<XsWatchEntry>, IoError> {
        self.check_watch()
    }

    fn unwatch(&self, path: &str, token: &str) -> Result<(), IoError> {
        self.unwatch(path, token)
    }

    fn get_stream(&self) -> Result<Self::XsStreamType<'_>, IoError> {
        self.get_stream()
    }
}

impl XsStreamTrait<'_> for XsStream<'_> {}
