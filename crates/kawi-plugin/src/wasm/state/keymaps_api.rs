use super::super::bindings::kawi::plugin::editor_keymaps::{
    self, KeyMode as WitKeyMode, KeymapSpec,
};
use super::super::bindings::kawi::plugin::types::Error;
use super::super::convert::error_from_rpc;
use super::{decode_json, serialize_params, WasmState};
use crate::protocol::{KeyMode, KeymapRemoveParams, KeymapSetParams, RegisteredKeymap};

impl editor_keymaps::Host for WasmState {
    fn set(&mut self, spec: KeymapSpec) -> Result<(), Error> {
        let params = KeymapSetParams {
            mode: key_mode(spec.mode),
            keys: spec.keys,
            command: spec.command,
            description: spec.description,
        };
        self.bridge
            .request("keymap.set", serialize_params(params)?)
            .map_err(error_from_rpc)?;
        Ok(())
    }

    fn remove(&mut self, mode: WitKeyMode, keys: String) -> Result<(), Error> {
        let params = serialize_params(KeymapRemoveParams {
            mode: key_mode(mode),
            keys,
        })?;
        self.bridge
            .request("keymap.remove", params)
            .map_err(error_from_rpc)?;
        Ok(())
    }

    fn list_registered(&mut self) -> Result<Vec<KeymapSpec>, Error> {
        let value = self
            .bridge
            .request("keymap.list", serde_json::Value::Null)
            .map_err(error_from_rpc)?;
        let keymaps: Vec<RegisteredKeymap> = decode_json(value)?;
        Ok(keymaps.into_iter().map(keymap_spec).collect())
    }
}

fn keymap_spec(keymap: RegisteredKeymap) -> KeymapSpec {
    KeymapSpec {
        mode: key_mode_to_wit(keymap.mode),
        keys: keymap.keys,
        command: keymap.command,
        description: keymap.description,
    }
}

fn key_mode(mode: WitKeyMode) -> KeyMode {
    match mode {
        WitKeyMode::Normal => KeyMode::Normal,
        WitKeyMode::Insert => KeyMode::Insert,
        WitKeyMode::Visual => KeyMode::Visual,
    }
}

fn key_mode_to_wit(mode: KeyMode) -> WitKeyMode {
    match mode {
        KeyMode::Normal => WitKeyMode::Normal,
        KeyMode::Insert => WitKeyMode::Insert,
        KeyMode::Visual => WitKeyMode::Visual,
    }
}
