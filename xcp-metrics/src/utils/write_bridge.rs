//! Wrapper for [`std::io::Write`] to implement [`std::fmt::Write`]

use std::{fmt, io};

pub struct WriterWrapper<'a, W: io::Write>(pub &'a mut W);

impl<W: io::Write> fmt::Write for WriterWrapper<'_, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_all(s.as_bytes()).map_err(|_| fmt::Error)
    }
}
