use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::io::{BufRead, Read, Write};

const MAX_HEADER_LINE: usize = 8 * 1024;
const MAX_HEADER_BYTES: usize = 32 * 1024;
const INITIAL_BODY_CAPACITY: usize = 8 * 1024;
pub const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

pub fn read_message(reader: &mut impl BufRead) -> Result<Option<Value>> {
    let Some(length) = read_headers(reader)? else {
        return Ok(None);
    };
    if length > MAX_MESSAGE_BYTES {
        bail!("jsonrpc message exceeds {MAX_MESSAGE_BYTES} bytes");
    }
    let mut body = Vec::with_capacity(length.min(INITIAL_BODY_CAPACITY));
    reader
        .by_ref()
        .take(length as u64)
        .read_to_end(&mut body)
        .context("read jsonrpc message body")?;
    if body.len() != length {
        bail!("unexpected end of stream in jsonrpc message body");
    }
    let value = serde_json::from_slice(&body).context("parse jsonrpc message json")?;
    Ok(Some(value))
}

pub fn write_message(writer: &mut impl Write, message: &Value) -> Result<()> {
    let body = serde_json::to_vec(message).context("serialize jsonrpc message")?;
    if body.len() > MAX_MESSAGE_BYTES {
        bail!("jsonrpc message exceeds {MAX_MESSAGE_BYTES} bytes");
    }
    write!(writer, "Content-Length: {}\r\n\r\n", body.len()).context("write jsonrpc header")?;
    writer.write_all(&body).context("write lsp body")?;
    writer.flush().context("flush jsonrpc message")?;
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
            bail!("unexpected end of stream in jsonrpc headers");
        };
        if line.is_empty() {
            break;
        }
        total = total.saturating_add(line.len());
        if total > MAX_HEADER_BYTES {
            bail!("jsonrpc headers exceed {MAX_HEADER_BYTES} bytes");
        }
        let Some((name, value)) = line.split_once(':') else {
            bail!("invalid jsonrpc header line");
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
        .by_ref()
        .take((MAX_HEADER_LINE + 1) as u64)
        .read_until(b'\n', &mut line)
        .context("read jsonrpc header")?;
    if read == 0 {
        return Ok(None);
    }
    if line.len() > MAX_HEADER_LINE {
        bail!("jsonrpc header line exceeds {MAX_HEADER_LINE} bytes");
    }
    let line = std::str::from_utf8(&line).context("jsonrpc header is not utf-8")?;
    Ok(Some(line.trim_end_matches(['\r', '\n']).to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::{self, BufRead, Cursor, Read};

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
    fn rejects_truncated_large_body() {
        let header = format!("Content-Length: {MAX_MESSAGE_BYTES}\r\n\r\n");
        let mut reader = Cursor::new(header.into_bytes());
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn round_trips_large_float_message() {
        let body = b"7010e090";
        let input = [
            format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes(),
            body,
        ]
        .concat();
        let mut reader = Cursor::new(input);
        let value = read_message(&mut reader).unwrap().expect("message");
        let mut encoded = Vec::new();
        write_message(&mut encoded, &value).unwrap();
        let mut reader = Cursor::new(encoded);
        assert!(read_message(&mut reader).unwrap().is_some());
    }

    #[test]
    fn rejects_large_claim_without_large_read() {
        let header = format!("Content-Length: {MAX_MESSAGE_BYTES}\r\n\r\n");
        let mut reader = TrackingReader::new(header.into_bytes());
        assert!(read_message(&mut reader).is_err());
        assert!(reader.max_read <= INITIAL_BODY_CAPACITY);
    }

    #[test]
    fn rejects_oversized_header_line() {
        let mut input = vec![b'a'; MAX_HEADER_LINE + 2];
        input.push(b'\n');
        let mut reader = Cursor::new(input);
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn rejects_oversized_messages() {
        let header = format!("Content-Length: {}\r\n\r\n", MAX_MESSAGE_BYTES + 1);
        let mut reader = Cursor::new(header.into_bytes());
        assert!(read_message(&mut reader).is_err());
    }

    #[test]
    fn rejects_oversized_outgoing_messages() {
        let message = Value::String("x".repeat(MAX_MESSAGE_BYTES));
        let mut buffer = Vec::new();
        assert!(write_message(&mut buffer, &message).is_err());
        assert!(buffer.is_empty());
    }

    struct TrackingReader {
        inner: Cursor<Vec<u8>>,
        max_read: usize,
    }

    impl TrackingReader {
        fn new(data: Vec<u8>) -> Self {
            Self {
                inner: Cursor::new(data),
                max_read: 0,
            }
        }
    }

    impl Read for TrackingReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            self.max_read = self.max_read.max(buffer.len());
            self.inner.read(buffer)
        }
    }

    impl BufRead for TrackingReader {
        fn fill_buf(&mut self) -> io::Result<&[u8]> {
            self.inner.fill_buf()
        }

        fn consume(&mut self, amount: usize) {
            self.inner.consume(amount);
        }
    }
}
