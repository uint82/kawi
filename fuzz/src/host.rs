use std::collections::BTreeMap;

use kawi_buffer::{apply_text_edits, char_offset_at, validate_text_edits, TextEdit};
use kawi_plugin::{
    host_method_capability, is_host_method_allowed, normalize_result, sanitize,
    validate_diagnostics, ApplyEditParams, Capability, CapabilityRegistry, EffectiveCapabilities,
    GetStateParams, MethodCapability, Outcome, PluginError, ProtocolTextEdit, RpcError, Scope,
    SetDiagnosticsParams, UiMessageParams,
};
use ropey::Rope;
use serde_json::{json, Value};

const PLUGIN: &str = "fuzz";

pub struct MockEditor {
    buffer: u64,
    revision: u64,
    text: Rope,
    effects: usize,
}

impl MockEditor {
    pub fn new() -> Self {
        Self {
            buffer: 1,
            revision: 0,
            text: Rope::from_str("é\n"),
            effects: 0,
        }
    }

    pub fn effects(&self) -> usize {
        self.effects
    }

    pub fn dispatch(
        &mut self,
        registry: &CapabilityRegistry,
        method: &str,
        params: Value,
    ) -> Outcome {
        if !is_host_method_allowed(registry, PLUGIN, method) {
            return Outcome::denied(denied_capability(method));
        }
        let result = self.handle(method, params);
        normalize_result(&result.map_err(PluginError::Rpc))
    }

    fn handle(&mut self, method: &str, params: Value) -> Result<Value, RpcError> {
        match method {
            "editor.get_state" => self.get_state(params),
            "editor.apply_edit" => self.apply_edit(params),
            "editor.set_diagnostics" => self.set_diagnostics(params),
            "ui.message" => self.message(params),
            _ => Err(RpcError::method_not_found(method)),
        }
    }

    fn get_state(&self, params: Value) -> Result<Value, RpcError> {
        let params: GetStateParams = serde_json::from_value(params).map_err(invalid_params)?;
        Ok(json!({
            "buffer": self.buffer,
            "revision": self.revision,
            "text": params.include_text.then(|| self.text.to_string()),
        }))
    }

    fn apply_edit(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: ApplyEditParams = serde_json::from_value(params).map_err(invalid_params)?;
        self.check_target(params.buffer, params.revision)?;
        let edits = self.protocol_edits(&params.edits)?;
        if edits.is_empty() {
            return Ok(json!({"revision": self.revision}));
        }
        validate_text_edits(self.text.len_chars(), &edits).map_err(invalid_edits)?;
        self.text = apply_text_edits(&self.text, &edits).map_err(invalid_edits)?;
        assert!(std::str::from_utf8(self.text.to_string().as_bytes()).is_ok());
        self.revision += 1;
        self.effects += 1;
        Ok(json!({"revision": self.revision}))
    }

    fn set_diagnostics(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: SetDiagnosticsParams =
            serde_json::from_value(params).map_err(invalid_params)?;
        self.check_target(params.buffer, params.revision)?;
        validate_diagnostics(&params.diagnostics)?;
        for diagnostic in &params.diagnostics {
            assert_sanitized(&diagnostic.message);
            if let Some(code) = &diagnostic.code {
                assert_sanitized(code);
            }
        }
        self.effects += 1;
        Ok(Value::Null)
    }

    fn message(&mut self, params: Value) -> Result<Value, RpcError> {
        let params: UiMessageParams = serde_json::from_value(params).map_err(invalid_params)?;
        assert_sanitized(&params.message);
        self.effects += 1;
        Ok(Value::Null)
    }

    fn check_target(&self, buffer: u64, revision: u64) -> Result<(), RpcError> {
        if buffer != self.buffer {
            return Err(RpcError::stale_buffer(buffer, self.buffer));
        }
        if revision != self.revision {
            return Err(RpcError::stale_revision(revision, self.revision));
        }
        Ok(())
    }

    fn protocol_edits(&self, edits: &[ProtocolTextEdit]) -> Result<Vec<TextEdit>, RpcError> {
        edits
            .iter()
            .map(|edit| {
                let start = self.char_offset(edit.start.line, edit.start.byte_column)?;
                let end = self.char_offset(edit.end.line, edit.end.byte_column)?;
                if start > end {
                    return Err(RpcError::invalid_position(
                        "edit start is after edit end".to_string(),
                    ));
                }
                Ok(TextEdit {
                    start,
                    end,
                    text: edit.text.clone(),
                })
            })
            .collect()
    }

    fn char_offset(&self, line: usize, byte_column: usize) -> Result<usize, RpcError> {
        char_offset_at(&self.text, line, byte_column)
            .map_err(|error| RpcError::invalid_position(error.to_string()))
    }
}

pub fn registry(grants: u16) -> CapabilityRegistry {
    let mut effective = EffectiveCapabilities::default();
    for (index, capability) in Capability::ALL.iter().enumerate() {
        if grants & (1 << index) != 0 {
            effective.granted.insert(*capability, Scope::Unscoped);
        }
    }
    let registry = CapabilityRegistry::default();
    registry.replace(BTreeMap::from([(PLUGIN.to_string(), effective)]));
    registry
}

fn denied_capability(method: &str) -> &'static str {
    match host_method_capability(method) {
        MethodCapability::Required(capability) => capability.name(),
        MethodCapability::None | MethodCapability::Unknown => "unknown",
    }
}

fn assert_sanitized(text: &str) {
    let sanitized = sanitize(text);
    assert!(!sanitized.chars().any(char::is_control));
}

fn invalid_params(error: serde_json::Error) -> RpcError {
    RpcError::invalid_params(error.to_string())
}

fn invalid_edits(error: kawi_buffer::TextEditError) -> RpcError {
    RpcError::invalid_params(format!("invalid text edits: {error:?}"))
}

#[cfg(test)]
mod tests;
