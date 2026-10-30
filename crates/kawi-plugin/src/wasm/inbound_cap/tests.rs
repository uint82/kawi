use std::time::Duration;

use bytes::Bytes;
use wasmtime_wasi::p2::{InputStream as InputStreamTrait, StreamError};
use wasmtime_wasi::runtime::in_tokio;

use super::*;
use crate::wasm::byte_budget::MAX_BYTES_PER_CONNECTION;
use crate::wasm::test_streams::SourceStream;

fn capped(data: Bytes, session: u64) -> CappedInputStream {
    let deadline = CallDeadline::new(Duration::from_secs(1));
    CappedInputStream::new(
        Box::new(SourceStream::new(data)),
        deadline,
        SessionBudget::with_limit(session),
    )
}

#[test]
fn read_counts_bytes() {
    let mut stream = capped(Bytes::from_static(b"abcdef"), MAX_BYTES_PER_CONNECTION);
    assert_eq!(&*stream.read(2).unwrap(), b"ab");
    assert_eq!(&*stream.read(10).unwrap(), b"cdef");
    assert!(stream.read(1).unwrap().is_empty());
}

#[test]
fn read_stops_at_the_connection_cap() {
    let data = Bytes::from(vec![0_u8; MAX_BYTES_PER_CONNECTION as usize + 1]);
    let mut stream = capped(data, MAX_BYTES_PER_CONNECTION);
    assert_eq!(
        stream
            .read(MAX_BYTES_PER_CONNECTION as usize + 1)
            .unwrap()
            .len(),
        MAX_BYTES_PER_CONNECTION as usize
    );
    assert!(matches!(stream.read(1), Err(StreamError::Closed)));
}

#[test]
fn read_stops_at_the_session_cap() {
    let mut stream = capped(Bytes::from_static(b"abcdefghij"), 4);
    assert_eq!(&*stream.read(10).unwrap(), b"abcd");
    assert!(matches!(stream.read(1), Err(StreamError::Closed)));
}

#[test]
fn skip_counts_bytes() {
    let mut stream = capped(Bytes::from_static(b"abcdefghij"), 4);
    assert_eq!(stream.skip(4).unwrap(), 4);
    assert!(matches!(stream.read(1), Err(StreamError::Closed)));
}

#[test]
fn zero_read_does_not_consume_budget() {
    let mut stream = capped(Bytes::from_static(b"ab"), 2);
    assert!(stream.read(0).unwrap().is_empty());
    assert_eq!(&*stream.read(2).unwrap(), b"ab");
    assert!(matches!(stream.read(1), Err(StreamError::Closed)));
}

#[test]
fn blocking_read_counts_bytes() {
    let mut stream = capped(Bytes::from_static(b"abc"), MAX_BYTES_PER_CONNECTION);
    let bytes = in_tokio(async { stream.blocking_read(2).await }).unwrap();
    assert_eq!(&*bytes, b"ab");
}
