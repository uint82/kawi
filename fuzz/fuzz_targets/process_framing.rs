#![no_main]

use std::io::{self, BufRead, Cursor, Read};

use kawi_jsonrpc::framing::{read_message, write_message, MAX_MESSAGE_BYTES};
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

const READ_BOUND: usize = 8 * 1024;

fuzz_target!(|data: &[u8]| {
    let mut reader = TrackingReader::new(data);
    let decoded = read_message(&mut reader);
    let limit = data.len().saturating_mul(2).max(READ_BOUND);
    assert!(
        reader.max_read <= limit,
        "read buffer {} exceeds {} for {} input bytes",
        reader.max_read,
        limit,
        data.len()
    );

    if let Ok(Some(value)) = decoded {
        let mut encoded = Vec::new();
        write_message(&mut encoded, &value).expect("round trip");
        assert!(encoded.len() <= MAX_MESSAGE_BYTES + 64);
        let mut round_trip = Cursor::new(encoded);
        assert!(read_message(&mut round_trip).expect("decode").is_some());
    }

    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        let mut encoded = Vec::new();
        let _ = write_message(&mut encoded, &value);
    }
});

struct TrackingReader {
    inner: Cursor<Vec<u8>>,
    max_read: usize,
}

impl TrackingReader {
    fn new(data: &[u8]) -> Self {
        Self {
            inner: Cursor::new(data.to_vec()),
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
