use std::path::Path;

use kawi_plugin::{
    ApplyEditParams, BufferGetParams, BufferInfo, BufferOpenParams, BufferState, Capability,
    RpcError, INTERNAL_ERROR,
};
use serde_json::Value;

use super::super::super::super::App;
use super::super::{invalid_params, json_result};
use super::ensure_active_buffer;

impl App {
    pub(super) fn plugin_buffer_list(&self) -> Result<Value, RpcError> {
        let infos = self
            .editor
            .buffer_infos()
            .into_iter()
            .map(|info| BufferInfo {
                buffer: info.number as u64,
                revision: info.revision,
                modified: self.editor.buffer_is_dirty(info.number).unwrap_or(false),
                path: info.path,
                language: info.language,
            })
            .collect::<Vec<_>>();
        json_result(infos)
    }

    pub(super) fn plugin_buffer_get(&self, params: Value) -> Result<Value, RpcError> {
        let params: BufferGetParams = serde_json::from_value(params).map_err(invalid_params)?;
        let state = self
            .buffer_state(params.buffer, params.include_text)
            .ok_or_else(|| RpcError::buffer_not_found(params.buffer))?;
        json_result(state)
    }

    pub(super) fn plugin_buffer_open(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: BufferOpenParams = serde_json::from_value(params).map_err(invalid_params)?;
        let Some(path) = self.plugins.resolve_buffer_path(plugin, &params.path) else {
            return Err(self
                .plugins
                .capability_error(plugin, Capability::EditorBuffersWrite));
        };
        self.open_file_location(Path::new(&path), None, None)
            .map_err(RpcError::invalid_params)?;
        let buffer = self.editor.buffer_number() as u64;
        let state = self
            .buffer_state(buffer, false)
            .ok_or_else(|| RpcError::buffer_not_found(buffer))?;
        json_result(state)
    }

    pub(super) fn plugin_buffer_apply_edit(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: ApplyEditParams = serde_json::from_value(params).map_err(invalid_params)?;
        ensure_active_buffer(self, params.buffer)?;
        let path = self
            .editor
            .buffer_path(self.editor.buffer_number())
            .ok_or_else(|| RpcError::buffer_not_active(params.buffer))?;
        if !self.plugins.allows_buffer_path(plugin, path) {
            return Err(self
                .plugins
                .capability_error(plugin, Capability::EditorBuffersWrite));
        }
        let params = serde_json::to_value(params)
            .map_err(|error| RpcError::new(INTERNAL_ERROR, error.to_string()))?;
        self.plugin_apply_edit(params)
    }

    fn buffer_state(&self, buffer: u64, include_text: bool) -> Option<BufferState> {
        let number = usize::try_from(buffer).ok()?;
        let info = self
            .editor
            .buffer_infos()
            .into_iter()
            .find(|info| info.number == number)?;
        Some(BufferState {
            buffer,
            revision: info.revision,
            modified: self.editor.buffer_is_dirty(number)?,
            text: include_text
                .then(|| self.editor.buffer_text(number))
                .flatten(),
            path: info.path,
            language: info.language,
        })
    }
}
