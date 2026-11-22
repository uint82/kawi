use super::super::bindings::kawi::plugin::types::{Edit, Error};
use super::super::convert::{edits_to_protocol, error_from_rpc};
use super::{decode_json, serialize_params, WasmState};
use crate::protocol::{ApplyEditParams, ApplyEditResult};

pub(super) const EDITOR_METHOD: &str = "editor.apply_edit";
pub(super) const BUFFER_METHOD: &str = "buffer.apply_edit";

pub(super) struct Request {
    pub(super) method: &'static str,
    pub(super) buffer: u64,
    pub(super) revision: u64,
    pub(super) edits: Vec<Edit>,
}

pub(super) fn send(state: &mut WasmState, request: Request) -> Result<u64, Error> {
    let edits = edits_to_protocol(request.edits).map_err(error_from_rpc)?;
    let params = serialize_params(ApplyEditParams {
        buffer: request.buffer,
        revision: request.revision,
        edits,
    })?;
    let value = state
        .bridge
        .request(request.method, params)
        .map_err(error_from_rpc)?;
    decode_json::<ApplyEditResult>(value).map(|result| result.revision)
}
