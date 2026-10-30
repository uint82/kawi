#![no_main]

use std::io::Cursor;

use kawi_jsonrpc::framing::{read_message, write_message, MAX_MESSAGE_BYTES};
use libfuzzer_sys::fuzz_target;
use serde_json::Value;

fuzz_target!(|data: &[u8]| {
    let mut reader = Cursor::new(data);
    if let Ok(Some(value)) = read_message(&mut reader) {
        let mut encoded = Vec::new();
        write_message(&mut encoded, &value).expect("round trip");
        assert!(encoded.len() <= MAX_MESSAGE_BYTES + 64);
        let mut round_trip = Cursor::new(encoded);
        assert_eq!(read_message(&mut round_trip).expect("decode"), Some(value));
    }

    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        let mut encoded = Vec::new();
        let _ = write_message(&mut encoded, &value);
    }
});
