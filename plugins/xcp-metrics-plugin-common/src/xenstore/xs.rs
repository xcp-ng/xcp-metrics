use std::io::Error as IoError;

pub use xenstore_rs::{XBTransaction, Xs, XsOpenFlags};

pub trait XsTrait: 'static + Sized {
    fn directory(&self, transaction: XBTransaction, path: &str) -> Result<Vec<String>, IoError>;

    fn read(&self, transaction: XBTransaction, path: &str) -> Result<String, IoError>;

    fn write(&self, transaction: XBTransaction, path: &str, data: &str) -> Result<(), IoError>;

    fn rm(&self, transaction: XBTransaction, path: &str) -> Result<(), IoError>;
}

impl XsTrait for Xs {
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
}
