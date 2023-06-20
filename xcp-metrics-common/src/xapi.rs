use std::{io, os::unix::net::UnixStream, path::Path};

const XAPI_SOCKET_PATH: &str = "/var/lib/xcp/";

pub fn connect_to_socket(name: &str) -> Result<UnixStream, io::Error> {
    UnixStream::connect(Path::new(XAPI_SOCKET_PATH).join(name))
}
