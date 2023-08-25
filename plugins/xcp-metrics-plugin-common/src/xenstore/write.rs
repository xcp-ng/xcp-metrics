
use std::io::Error as IoError;

use super::xs::XBTransaction;

pub trait XsWrite {
  fn write(&self, transaction: XBTransaction, path: &str, data: &str) -> Result<(), IoError>;
  
  fn rm(&self, transaction: XBTransaction, path: &str) -> Result<(), IoError>;
}

#[cfg(feature = "xenstore-wip")]
impl XsWrite for super::xs::Xs {
    fn write(&self, transaction: XBTransaction, path: &str, data: &str) -> Result<(), IoError> {
        self.write(transaction, path, data)
    }

    fn rm(&self, transaction: XBTransaction, path: &str) -> Result<(), IoError> {
        self.rm(transaction, path)
    }
}
