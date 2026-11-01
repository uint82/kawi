use kawi_plugin::{
    validate_decorations, validate_statusline, RpcError, SetDecorationsParams, SetStatuslineParams,
};
use serde_json::Value;

use super::super::App;
use super::host_methods::invalid_params;

impl App {
    pub(super) fn plugin_set_decorations(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: SetDecorationsParams =
            serde_json::from_value(params).map_err(invalid_params)?;
        validate_decorations(&params.decorations)?;
        let buffer = usize::try_from(params.buffer)
            .map_err(|_| RpcError::invalid_params("buffer does not fit usize"))?;
        let current = self.editor.buffer_revision_at(buffer).ok_or_else(|| {
            RpcError::stale_buffer(params.buffer, self.editor.buffer_number() as u64)
        })?;
        if params.revision != current {
            return Err(RpcError::stale_revision(params.revision, current));
        }
        let snapshot = self.editor.buffer_snapshot(buffer).ok_or_else(|| {
            RpcError::stale_buffer(params.buffer, self.editor.buffer_number() as u64)
        })?;
        self.plugin_decorations
            .set(plugin, buffer, current, &params.decorations, &snapshot)?;
        Ok(Value::Null)
    }

    pub(super) fn plugin_set_statusline(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: SetStatuslineParams = serde_json::from_value(params).map_err(invalid_params)?;
        validate_statusline(&params.segments)?;
        self.plugin_statusline.set(plugin, &params.segments);
        Ok(Value::Null)
    }
}
