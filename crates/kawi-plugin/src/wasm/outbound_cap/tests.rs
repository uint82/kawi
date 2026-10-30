use std::sync::{Arc, Mutex};
use std::time::Duration;

use wasmtime_wasi::p2::bindings::sync::io::streams::HostOutputStream as HostOutputStreamResource;
use wasmtime_wasi::p2::{DynInputStream, InputStream as InputStreamTrait};
use wasmtime_wasi::runtime::in_tokio;

use super::*;

struct RecordingStream {
    written: Arc<Mutex<Vec<u8>>>,
    permit: usize,
}

impl RecordingStream {
    fn new(permit: usize) -> (Self, Arc<Mutex<Vec<u8>>>) {
        let written = Arc::new(Mutex::new(Vec::new()));
        (
            Self {
                written: Arc::clone(&written),
                permit,
            },
            written,
        )
    }
}

#[async_trait]
impl OutputStreamTrait for RecordingStream {
    fn write(&mut self, bytes: Bytes) -> StreamResult<()> {
        self.written
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .extend_from_slice(&bytes);
        Ok(())
    }

    fn flush(&mut self) -> StreamResult<()> {
        Ok(())
    }

    fn check_write(&mut self) -> StreamResult<usize> {
        Ok(self.permit)
    }
}

#[async_trait]
impl Pollable for RecordingStream {
    async fn ready(&mut self) {}
}

struct SourceStream {
    data: Bytes,
}

#[async_trait]
impl InputStreamTrait for SourceStream {
    fn read(&mut self, size: usize) -> StreamResult<Bytes> {
        let len = self.data.len().min(size);
        Ok(self.data.split_to(len))
    }
}

#[async_trait]
impl Pollable for SourceStream {
    async fn ready(&mut self) {}
}

fn capped(permit: usize) -> (CappedOutputStream, Arc<Mutex<Vec<u8>>>) {
    let (inner, written) = RecordingStream::new(permit);
    let deadline = CallDeadline::new(Duration::from_secs(1));
    (CappedOutputStream::new(Box::new(inner), deadline), written)
}

#[test]
fn write_counts_bytes() {
    let (mut stream, written) = capped(64 * 1024);
    stream.check_write().unwrap();
    stream.write(Bytes::from_static(b"abc")).unwrap();
    assert_eq!(&*written.lock().unwrap(), b"abc");
}

#[test]
fn blocking_write_and_flush_counts_bytes() {
    let (mut stream, written) = capped(64 * 1024);
    let result = in_tokio(async {
        stream
            .blocking_write_and_flush(Bytes::from_static(b"abc"))
            .await
    });
    assert!(result.is_ok());
    assert_eq!(&*written.lock().unwrap(), b"abc");
}

#[test]
fn write_zeroes_counts_bytes() {
    let (mut stream, written) = capped(64 * 1024);
    stream.write_zeroes(16).unwrap();
    assert_eq!(written.lock().unwrap().len(), 16);
}

#[test]
fn blocking_write_zeroes_and_flush_counts_bytes() {
    let (stream, written) = capped(64 * 1024);
    let output: DynOutputStream = Box::new(stream);
    let mut table = ResourceTable::new();
    let output = table.push(output).unwrap();
    let result = HostOutputStreamResource::blocking_write_zeroes_and_flush(&mut table, output, 16);
    assert!(result.is_ok());
    assert_eq!(written.lock().unwrap().len(), 16);
}

#[test]
fn splice_counts_bytes() {
    let (stream, written) = capped(64 * 1024);
    let output: DynOutputStream = Box::new(stream);
    let source: DynInputStream = Box::new(SourceStream {
        data: Bytes::from_static(b"hello"),
    });
    let mut table = ResourceTable::new();
    let output = table.push(output).unwrap();
    let source = table.push(source).unwrap();
    let count = HostOutputStreamResource::splice(&mut table, output, source, 5).unwrap();
    assert_eq!(count, 5);
    assert_eq!(&*written.lock().unwrap(), b"hello");
}

#[test]
fn blocking_write_stops_at_the_cap() {
    let (mut stream, written) = capped(64 * 1024);
    let bytes = Bytes::from(vec![0_u8; MAX_OUTBOUND_BYTES_PER_CONNECTION as usize + 1]);
    let result = in_tokio(async { stream.blocking_write_and_flush(bytes).await });
    assert!(result.is_err());
    assert_eq!(
        written.lock().unwrap().len(),
        MAX_OUTBOUND_BYTES_PER_CONNECTION as usize
    );
}

#[test]
fn permits_at_most_the_remaining_budget() {
    let mut budget = OutboundBudget::new(10);
    assert_eq!(budget.permit(8).unwrap(), 8);
    budget.consume(8).unwrap();
    assert_eq!(budget.permit(8).unwrap(), 2);
}

#[test]
fn rejects_writes_above_the_permit() {
    let mut budget = OutboundBudget::new(10);
    budget.permit(4).unwrap();
    assert!(matches!(budget.consume(5), Err(StreamError::Trap(_))));
}

#[test]
fn allows_one_drain_check_after_exhaustion() {
    let mut budget = OutboundBudget::new(4);
    budget.permit(4).unwrap();
    budget.consume(4).unwrap();
    assert_eq!(budget.permit(1).unwrap(), 1);
    assert!(matches!(budget.permit(1), Err(StreamError::Closed)));
}

#[test]
fn permits_zero_writes_without_consuming_budget() {
    let mut budget = OutboundBudget::new(4);
    budget.permit(4).unwrap();
    budget.consume(0).unwrap();
    assert_eq!(budget.permit(4).unwrap(), 4);
}
