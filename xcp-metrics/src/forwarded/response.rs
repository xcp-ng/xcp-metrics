use std::{fmt::Debug, io::Write};

use xcp_metrics_common::xapi::hyper::http::Response;

/// Write the HTTP response into some writer.
pub async fn write_response<W, B>(
    writer: &mut W,
    mut response: Response<B>,
) -> Result<(), anyhow::Error>
where
    W: Write + Debug,
    B: AsRef<[u8]> + Debug,
{
    tracing::trace!("Sending HTTP response {response:?} to {writer:?}");

    write!(
        writer,
        "HTTP/1.1 {} {}\r\n",
        response.status().as_u16(),
        response.status().canonical_reason().unwrap_or_default()
    )?;

    // Add content-length if not defined
    if !response.headers().contains_key("content-length") {
        let body_length = response.body().as_ref().len();
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
    writer.write_all(response.body().as_ref())?;

    Ok(())
}
