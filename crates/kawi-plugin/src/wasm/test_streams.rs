use std::sync::{Arc, Mutex};

use bytes::Bytes;
use wasmtime_wasi::async_trait;
use wasmtime_wasi::p2::{
    InputStream as InputStreamTrait, OutputStream as OutputStreamTrait, Pollable, StreamResult,
};

pub(crate) struct RecordingStream {
    written: Arc<Mutex<Vec<u8>>>,
    permit: usize,
}

impl RecordingStream {
    pub(crate) fn new(permit: usize) -> (Self, Arc<Mutex<Vec<u8>>>) {
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

pub(crate) struct SourceStream {
    data: Bytes,
}

impl SourceStream {
    pub(crate) fn new(data: Bytes) -> Self {
        Self { data }
    }
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
