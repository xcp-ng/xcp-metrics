//! [write_response] implementation
use std::{fmt::Debug, io::Write};

use xapi::hyper::{body, http::Response, Body};

/// Write the HTTP response into some writer.
pub async fn write_response<W>(
    writer: &mut W,
    mut response: Response<Body>,
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

    let body = body::to_bytes(response.body_mut()).await?;

    // Add content-length if not defined
    if !response.headers().contains_key("content-length") {
        let body_length = body.len();
        response
            .headers_mut()
            .insert("content-length", body_length.into());
    }

    for (name, value) in response.headers() {
        write!(
            writer,
            "{}: {}\r\n",
            name.as_str(),
            String::from_utf8_lossy(value.as_bytes())
        )?;
    }

    write!(writer, "\r\n")?;
    writer.write_all(&body)?;

    Ok(())
}
