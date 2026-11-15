use super::super::bindings::kawi::plugin::editor_autocmds::{self, AutocmdInfo, AutocmdSpec};
use super::super::bindings::kawi::plugin::types::Error;
use super::super::convert::error_from_rpc;
use super::{decode_json, serialize_params, WasmState};
use crate::protocol::{AutocmdRemoveParams, RegisteredAutocmd, RpcError, PLUGIN_UNAVAILABLE};

impl editor_autocmds::Host for WasmState {
    fn register(&mut self, spec: AutocmdSpec) -> Result<AutocmdInfo, Error> {
        let params = crate::protocol::AutocmdSpec {
            event: spec.event,
            pattern: spec.pattern,
            buffer: spec.buffer,
            once: spec.once,
        };
        let value = self
            .bridge
            .request("autocmd.register", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        autocmd_info_from_json(value)
    }

    fn remove(&mut self, id: u64) -> Result<(), Error> {
        let params = serialize_params(AutocmdRemoveParams { id })?;
        self.bridge
            .request("autocmd.remove", params)
            .map_err(error_from_rpc)?;
        Ok(())
    }

    fn list_registered(&mut self) -> Result<Vec<AutocmdInfo>, Error> {
        let value = self
            .bridge
            .request("autocmd.list", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        let autocmds: Vec<RegisteredAutocmd> = decode_json(value)?;
        Ok(autocmds.into_iter().map(autocmd_info).collect())
    }

    fn clear(&mut self) -> Result<(), Error> {
        self.bridge
            .request("autocmd.clear", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        Ok(())
    }

    fn emit_user_event(&mut self, name: String, payload: Option<String>) -> Result<(), Error> {
        let params = crate::protocol::UserEventParams { name, payload };
        self.bridge
            .request("editor.emit_user_event", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        Ok(())
    }
}

fn autocmd_info_from_json(value: serde_json::Value) -> Result<AutocmdInfo, Error> {
    let autocmd: RegisteredAutocmd = serde_json::from_value(value)
        .map_err(|error| error_from_rpc(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string())))?;
    Ok(autocmd_info(autocmd))
}

fn autocmd_info(autocmd: RegisteredAutocmd) -> AutocmdInfo {
    AutocmdInfo {
        id: autocmd.id,
        event: autocmd.event,
        pattern: autocmd.pattern,
        buffer: autocmd.buffer,
        once: autocmd.once,
    }
}
