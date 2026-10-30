#![no_main]

use kawi_plugin::fuzz::{stream_reads, stream_writes, FuzzWriteOp, CONNECTION_CAP};
use libfuzzer_sys::fuzz_target;

const DEFAULT_SESSION: u64 = 64 * 1024 * 1024;
const DEFAULT_PERMIT: usize = CONNECTION_CAP as usize;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let mut session = DEFAULT_SESSION;
    let mut permit = DEFAULT_PERMIT;
    let mut source = Vec::new();
    let mut sizes = Vec::new();
    let mut ops = Vec::new();

    for line in text.lines() {
        let mut parts = line.split_whitespace();
        match parts.next() {
            Some("session") => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<u64>().ok()) {
                    session = value;
                }
            }
            Some("permit") => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<usize>().ok()) {
                    permit = value.min(DEFAULT_PERMIT);
                }
            }
            Some("data") => {
                if let Some(value) = parts.next() {
                    source = decode_hex(value);
                }
            }
            Some("r") => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<usize>().ok()) {
                    sizes.push(value.min(DEFAULT_PERMIT.saturating_add(1)));
                }
            }
            Some("c") => ops.push(FuzzWriteOp::CheckWrite),
            Some("w") => {
                if let Some(value) = parts.next() {
                    ops.push(FuzzWriteOp::Write(decode_hex(value)));
                }
            }
            Some("z") => {
                if let Some(value) = parts.next().and_then(|value| value.parse::<usize>().ok()) {
                    ops.push(FuzzWriteOp::WriteZeroes(
                        value.min(DEFAULT_PERMIT.saturating_add(1)),
                    ));
                }
            }
            Some("f") => ops.push(FuzzWriteOp::Flush),
            _ => {}
        }
    }

    let reads = stream_reads(&source, session, &sizes);
    assert_eq!(reads.results.len(), sizes.len());
    assert!(reads.consumed <= source.len());
    assert!(reads.consumed as u64 <= CONNECTION_CAP);
    assert!(reads.consumed as u64 <= session);
    assert_eq!(reads.session_remaining, session - reads.consumed as u64);

    let writes = stream_writes(session, permit, &ops);
    assert_eq!(writes.results.len(), ops.len());
    assert!(writes.written as u64 <= CONNECTION_CAP);
    assert!(writes.written as u64 <= session);
    assert_eq!(writes.session_remaining, session - writes.written as u64);
    let requested = ops
        .iter()
        .map(|op| match op {
            FuzzWriteOp::Write(bytes) => bytes.len(),
            FuzzWriteOp::WriteZeroes(len) => *len,
            FuzzWriteOp::CheckWrite | FuzzWriteOp::Flush => 0,
        })
        .fold(0_usize, usize::saturating_add);
    assert!(writes.written <= requested);
});

fn decode_hex(value: &str) -> Vec<u8> {
    let bytes = value.as_bytes();
    bytes
        .chunks_exact(2)
        .filter_map(|pair| {
            let high = hex_digit(pair[0])?;
            let low = hex_digit(pair[1])?;
            Some(high << 4 | low)
        })
        .collect()
}

fn hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}
