use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::io::{BufRead, Write};

const MAX_HEADER_LINE: usize = 8 * 1024;
const MAX_HEADER_BYTES: usize = 32 * 1024;
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

pub fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>> {
    let Some(length) = read_headers(reader)? else {
        return Ok(None);
    };
    if length > MAX_MESSAGE_BYTES {
        bail!("lsp message exceeds {MAX_MESSAGE_BYTES} bytes");
    }
    let mut body = vec![0; length];
    reader
        .read_exact(&mut body)
        .context("read lsp message body")?;
    let value = serde_json::from_slice(&body).context("parse lsp message json")?;
    Ok(Some(value))
}

pub fn write_message(writer: &mut impl Write, message: &Value) -> Result<()> {
    let body = serde_json::to_vec(message).context("serialize lsp message")?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len()).context("write lsp header")?;
    writer.write_all(&body).context("write lsp body")?;
    writer.flush().context("flush lsp message")?;
    Ok(())
}

fn read_headers(reader: &mut impl BufRead) -> Result<Option<usize>> {
    let mut content_length = None;
    let mut total: usize = 0;
    loop {
        let line = read_header_line(reader)?;
        let Some(line) = line else {
            if content_length.is_none() {
                return Ok(None);
            }
            bail!("unexpected end of stream in lsp headers");
        };
        if line.is_empty() {
            break;
        }
        total = total.saturating_add(line.len());
        if total > MAX_HEADER_BYTES {
            bail!("lsp headers exceed {MAX_HEADER_BYTES} bytes");
        }
        let Some((name, value)) = line.split_once(':') else {
            bail!("invalid lsp header line");
        };
        if name.eq_ignore_ascii_case("content-length") {
            if content_length.is_some() {
                bail!("duplicate Content-Length header");
            }
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .context("invalid Content-Length header")?,
            );
        }
    }
    content_length
        .map(Some)
        .ok_or_else(|| anyhow::anyhow!("missing Content-Length header"))
}

fn read_header_line(reader: &mut impl BufRead) -> Result<Option<String>> {
    let mut line = Vec::new();
    let read = reader
        .read_until(b'\n', &mut line)
        .context("read lsp header")?;
    if read == 0 {
        return Ok(None);
    }
    if line.len() > MAX_HEADER_LINE {
        bail!("lsp header line exceeds {MAX_HEADER_LINE} bytes");
    }
    let line = std::str::from_utf8(&line).context("lsp header is not utf-8")?;
    Ok(Some(line.trim_end_matches(['\r', '\n']).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    #[test]
    fn round_trips_a_message() {
        let message = json!({"jsonrpc": "2.0", "method": "test"});
        let mut buffer = Vec::new();
        write_message(&mut buffer, &message).unwrap();
        let mut reader = Cursor::new(buffer);
        assert_eq!(read_message(&mut reader).unwrap(), Some(message));
    }

    #[test]
    fn reads_multiple_messages() {
        let first = json!({"one": 1});
        let second = json!({"two": 2});
        let mut buffer = Vec::new();
        write_message(&mut buffer, &first).unwrap();
        write_message(&mut buffer, &second).unwrap();
        let mut reader = Cursor::new(buffer);
        assert_eq!(read_message(&mut reader).unwrap(), Some(first));
        assert_eq!(read_message(&mut reader).unwrap(), Some(second));
        assert_eq!(read_message(&mut reader).unwrap(), None);
    }

    #[test]
    fn accepts_lowercase_headers() {
        let body = br#"{"jsonrpc":"2.0","method":"x"}"#;
        let input = format!("content-length: {}\r\n\r\n", body.len());
        let mut reader = Cursor::new([input.as_bytes(), body].concat());
        assert!(read_message(&mut reader).unwrap().is_some());
    }

    #[test]
    fn rejects_missing_content_length() {
        let mut reader = Cursor::new(b"Content-Type: application/json\r\n\r\n".to_vec());
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn rejects_duplicate_content_length() {
        let mut reader = Cursor::new(b"Content-Length: 2\r\nContent-Length: 2\r\n\r\n{}".to_vec());
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn rejects_truncated_body() {
        let mut reader = Cursor::new(b"Content-Length: 10\r\n\r\n{}".to_vec());
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn rejects_oversized_messages() {
        let header = format!("Content-Length: {}\r\n\r\n", MAX_MESSAGE_BYTES + 1);
        let mut reader = Cursor::new(header.into_bytes());
        assert!(read_message(&mut reader).is_err());
    }
}
