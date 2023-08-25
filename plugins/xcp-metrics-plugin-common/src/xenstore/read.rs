use std::io::Error as IoError;

use super::xs::XBTransaction;

pub trait XsRead {
    fn directory(&self, transaction: XBTransaction, path: &str) -> Result<Vec<String>, IoError>;
    fn read(&self, transaction: XBTransaction, path: &str) -> Result<String, IoError>;
}

impl XsRead for super::xs::Xs {
    fn directory(&self, transaction: XBTransaction, path: &str) -> Result<Vec<String>, IoError> {
        self.directory(transaction, path)
    }

    fn read(&self, transaction: XBTransaction, path: &str) -> Result<String, IoError> {
        self.read(transaction, path)
    }
}
