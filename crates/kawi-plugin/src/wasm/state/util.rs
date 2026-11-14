use wasmtime_wasi::p2::pipe::MemoryOutputPipe;

use super::super::bindings::kawi::plugin::types::Error;
use super::HostBridge;
use crate::protocol::{RpcError, PLUGIN_UNAVAILABLE};

pub(crate) fn decode_json<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
) -> Result<T, Error> {
    serde_json::from_value(value).map_err(|error| {
        super::super::convert::error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))
    })
}

pub(crate) fn serialize_params<T: serde::Serialize>(params: T) -> Result<serde_json::Value, Error> {
    serde_json::to_value(params).map_err(|error| {
        super::super::convert::error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))
    })
}

pub(super) fn drain(
    pipe: &MemoryOutputPipe,
    offset: usize,
    stream: &str,
    bridge: &HostBridge,
) -> usize {
    let contents = pipe.contents();
    if contents.len() <= offset {
        return offset;
    }
    let text = String::from_utf8_lossy(&contents[offset..]);
    for line in text.lines() {
        bridge.log(format!("{stream}: {line}"));
    }
    contents.len()
}
