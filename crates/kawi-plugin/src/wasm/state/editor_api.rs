use super::super::bindings::kawi::plugin::types::{
    BufferInfo as WitBufferInfo, BufferState as WitBufferState, Edit, Error,
    OptionInfo as WitOptionInfo, Position as WitPosition, State, WindowInfo as WitWindowInfo,
};
use super::super::bindings::kawi::plugin::{
    editor_buffers, editor_buffers_write, editor_control, editor_windows, options_read,
};
use super::super::convert::{
    buffer_infos_from_json, buffer_state_from_json, error_from_rpc, option_info_from_json,
    option_infos_from_json, position_to_protocol, selection_to_protocol, split_direction_name,
    window_info_from_json, window_infos_from_json,
};
use super::{apply_edit_request, serialize_params, WasmState};
use crate::protocol::{
    BufferGetParams, BufferOpenParams, OptionsGetParams, OptionsListParams, SetCursorParams,
    SetModeParams, SetSelectionParams, WindowCloseParams, WindowOpenParams, WindowParams,
};

impl editor_buffers::Host for WasmState {
    fn list_buffers(&mut self) -> Result<Vec<WitBufferInfo>, Error> {
        let value = self
            .bridge
            .request("buffer.list", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        buffer_infos_from_json(value).map_err(error_from_rpc)
    }

    fn get(&mut self, buffer: u64, include_text: bool) -> Result<WitBufferState, Error> {
        let params = serialize_params(BufferGetParams {
            buffer,
            include_text,
        })?;
        let value = self
            .bridge
            .request("buffer.get", params)
            .map_err(error_from_rpc)?;
        buffer_state_from_json(value).map_err(error_from_rpc)
    }
}

impl editor_buffers_write::Host for WasmState {
    fn open(&mut self, path: String) -> Result<WitBufferState, Error> {
        let params = serialize_params(BufferOpenParams { path })?;
        let value = self
            .bridge
            .request("buffer.open", params)
            .map_err(error_from_rpc)?;
        buffer_state_from_json(value).map_err(error_from_rpc)
    }

    fn apply_edit(&mut self, buffer: u64, revision: u64, edits: Vec<Edit>) -> Result<u64, Error> {
        apply_edit_request(self, buffer, revision, edits)
    }
}

impl editor_windows::Host for WasmState {
    fn list_windows(&mut self) -> Result<Vec<WitWindowInfo>, Error> {
        let value = self
            .bridge
            .request("window.list", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        window_infos_from_json(value).map_err(error_from_rpc)
    }

    fn get(&mut self, window: u64) -> Result<WitWindowInfo, Error> {
        let params = serialize_params(WindowParams { window })?;
        let value = self
            .bridge
            .request("window.get", params)
            .map_err(error_from_rpc)?;
        window_info_from_json(value).map_err(error_from_rpc)
    }

    fn focus(&mut self, window: u64) -> Result<WitWindowInfo, Error> {
        let params = serialize_params(WindowParams { window })?;
        let value = self
            .bridge
            .request("window.focus", params)
            .map_err(error_from_rpc)?;
        window_info_from_json(value).map_err(error_from_rpc)
    }

    fn open(
        &mut self,
        buffer: u64,
        direction: super::super::bindings::kawi::plugin::types::SplitDirection,
    ) -> Result<WitWindowInfo, Error> {
        let params = serialize_params(WindowOpenParams {
            buffer,
            direction: split_direction_name(direction).to_string(),
        })?;
        let value = self
            .bridge
            .request("window.open", params)
            .map_err(error_from_rpc)?;
        window_info_from_json(value).map_err(error_from_rpc)
    }

    fn close(&mut self, window: u64, force: bool) -> Result<WitWindowInfo, Error> {
        let params = serialize_params(WindowCloseParams { window, force })?;
        let value = self
            .bridge
            .request("window.close", params)
            .map_err(error_from_rpc)?;
        window_info_from_json(value).map_err(error_from_rpc)
    }
}

impl editor_control::Host for WasmState {
    fn set_cursor(&mut self, buffer: u64, cursor: WitPosition) -> Result<State, Error> {
        let params = serialize_params(SetCursorParams {
            buffer,
            cursor: position_to_protocol(cursor).map_err(error_from_rpc)?,
        })?;
        self.set_control_state("editor.set_cursor", params)
    }

    fn set_selection(
        &mut self,
        buffer: u64,
        selection: Option<super::super::bindings::kawi::plugin::types::Selection>,
    ) -> Result<State, Error> {
        let selection = selection
            .map(selection_to_protocol)
            .transpose()
            .map_err(error_from_rpc)?;
        let params = serialize_params(SetSelectionParams { buffer, selection })?;
        self.set_control_state("editor.set_selection", params)
    }

    fn set_mode(&mut self, buffer: u64, mode: String) -> Result<State, Error> {
        let params = serialize_params(SetModeParams { buffer, mode })?;
        self.set_control_state("editor.set_mode", params)
    }
}

impl options_read::Host for WasmState {
    fn list_options(&mut self, prefix: Option<String>) -> Result<Vec<WitOptionInfo>, Error> {
        let params = serialize_params(OptionsListParams { prefix })?;
        let value = self
            .bridge
            .request("options.list", params)
            .map_err(error_from_rpc)?;
        option_infos_from_json(value).map_err(error_from_rpc)
    }

    fn get(&mut self, name: String) -> Result<WitOptionInfo, Error> {
        let params = serialize_params(OptionsGetParams { name })?;
        let value = self
            .bridge
            .request("options.get", params)
            .map_err(error_from_rpc)?;
        option_info_from_json(value).map_err(error_from_rpc)
    }
}
