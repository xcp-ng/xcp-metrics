//! [write_response] implementation
use std::{fmt::Debug, io::Write};

use http_body_util::{BodyExt, Full};
use hyper::{body::Bytes, http::Response};

/// Write the HTTP response into some writer.
pub async fn write_response<W>(
    writer: &mut W,
    response: Response<Full<Bytes>>,
) -> Result<(), anyhow::Error>
where
    W: Write + Debug,
{
    tracing::trace!("Sending HTTP response {response:?} to {writer:?}");

    write!(
        writer,
        "HTTP/1.1 {} {}\r\n",
        response.status().as_u16(),
        response.status().canonical_reason().unwrap_or_default()
    )?;

    let (mut parts, body) = response.into_parts();
    let body = body.collect().await?.to_bytes();

    // Add content-length if not defined
    if !parts.headers.contains_key("content-length") {
        let body_length = body.len();
        parts.headers.insert("content-length", body_length.into());
    }

    for (name, value) in parts.headers {
        if let Some(name) = name {
            write!(
                writer,
                "{}: {}\r\n",
                name.as_str(),
                String::from_utf8_lossy(value.as_bytes())
            )?;
        }
    }

    write!(writer, "\r\n")?;
    writer.write_all(&body)?;

    Ok(())
}
