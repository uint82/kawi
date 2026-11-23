use std::path::Path;

use kawi_plugin::{
    missing_event_capability, validate_autocmd_spec, AutocmdSpec, Capability, RegisteredAutocmd,
    MAX_PLUGIN_AUTOCMDS,
};
use serde_json::Value;

use super::PluginRuntime;
use crate::app::path_match::glob_matches;

#[derive(Debug)]
pub(crate) enum AutocmdError {
    Invalid(String),
    Capability(Capability),
}

#[derive(Debug, Clone)]
pub(super) struct AutocmdRegistration {
    pub(super) plugin: String,
    pub(super) id: u64,
    pub(super) event: String,
    pub(super) patterns: Option<Vec<String>>,
    pub(super) buffer: Option<u64>,
    pub(super) once: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AutocmdFire {
    pub(super) plugin: String,
    pub(super) id: u64,
}

impl PluginRuntime {
    pub(crate) fn register_autocmd(
        &mut self,
        plugin: &str,
        spec: AutocmdSpec,
    ) -> Result<RegisteredAutocmd, AutocmdError> {
        if !self.contains(plugin) {
            return Err(AutocmdError::Invalid(format!("unknown plugin: {plugin}")));
        }
        validate_autocmd_spec(&spec).map_err(AutocmdError::Invalid)?;
        if let Some(capability) = missing_event_capability(&self.capabilities, plugin, &spec.event)
        {
            return Err(AutocmdError::Capability(capability));
        }
        let count = self
            .autocmds
            .iter()
            .filter(|autocmd| autocmd.plugin == plugin)
            .count();
        if count >= MAX_PLUGIN_AUTOCMDS {
            return Err(AutocmdError::Invalid(format!(
                "at most {MAX_PLUGIN_AUTOCMDS} autocmds are allowed per plugin"
            )));
        }
        let id = self.next_autocmd_id;
        self.next_autocmd_id += 1;
        let patterns = spec
            .pattern
            .map(|pattern| pattern.split(',').map(str::to_string).collect());
        self.autocmds.push(AutocmdRegistration {
            plugin: plugin.to_string(),
            id,
            event: spec.event,
            patterns,
            buffer: spec.buffer,
            once: spec.once,
        });
        self.registered_autocmds(plugin)
            .into_iter()
            .find(|autocmd| autocmd.id == id)
            .ok_or_else(|| AutocmdError::Invalid("autocmd registration failed".to_string()))
    }

    pub(crate) fn remove_autocmd(&mut self, plugin: &str, id: u64) -> Result<(), String> {
        let Some(index) = self
            .autocmds
            .iter()
            .position(|autocmd| autocmd.plugin == plugin && autocmd.id == id)
        else {
            return Err(format!("autocmd is not registered: {id}"));
        };
        self.autocmds.remove(index);
        Ok(())
    }

    pub(crate) fn registered_autocmds(&self, plugin: &str) -> Vec<RegisteredAutocmd> {
        self.autocmds
            .iter()
            .filter(|autocmd| autocmd.plugin == plugin)
            .map(registered_autocmd)
            .collect()
    }

    pub(crate) fn clear_autocmds(&mut self, plugin: &str) {
        self.autocmds.retain(|autocmd| autocmd.plugin != plugin);
    }

    pub(crate) fn clear_all_autocmds(&mut self) {
        self.autocmds.clear();
    }

    pub(super) fn dispatch_autocmds(&mut self, event: &str, payload: &Value) -> Vec<AutocmdFire> {
        let target = pattern_target(event, payload);
        let buffer = payload.get("buffer").and_then(Value::as_u64);
        let mut fires = Vec::new();
        let mut retained = Vec::with_capacity(self.autocmds.len());
        for autocmd in self.autocmds.drain(..) {
            let matches = autocmd.event == event
                && autocmd_buffer_matches(autocmd.buffer, buffer)
                && autocmd_pattern_matches(autocmd.patterns.as_deref(), target);
            if !matches {
                retained.push(autocmd);
                continue;
            }
            fires.push(AutocmdFire {
                plugin: autocmd.plugin.clone(),
                id: autocmd.id,
            });
            if !autocmd.once {
                retained.push(autocmd);
            }
        }
        self.autocmds = retained;
        if event == "buffer_closed" {
            if let Some(buffer) = buffer {
                self.autocmds
                    .retain(|autocmd| autocmd.buffer != Some(buffer));
            }
        }
        fires
    }
}

fn registered_autocmd(autocmd: &AutocmdRegistration) -> RegisteredAutocmd {
    RegisteredAutocmd {
        id: autocmd.id,
        event: autocmd.event.clone(),
        pattern: autocmd.patterns.as_ref().map(|patterns| patterns.join(",")),
        buffer: autocmd.buffer,
        once: autocmd.once,
    }
}

fn autocmd_buffer_matches(registered: Option<u64>, payload: Option<u64>) -> bool {
    match registered {
        None => true,
        Some(registered) => payload == Some(registered),
    }
}

fn autocmd_pattern_matches(patterns: Option<&[String]>, target: Option<&str>) -> bool {
    let Some(patterns) = patterns else {
        return true;
    };
    let Some(target) = target else {
        return patterns.iter().any(|pattern| pattern == "*");
    };
    let name = Path::new(target)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(target);
    patterns.iter().any(|pattern| {
        pattern == "*" || glob_matches(pattern, target) || glob_matches(pattern, name)
    })
}

fn pattern_target<'a>(event: &str, payload: &'a Value) -> Option<&'a str> {
    match event {
        "buffer_opened" | "file_opened" | "buffer_closed" | "file_saved" => {
            payload.get("path").and_then(Value::as_str)
        }
        "language_changed" => payload.get("language").and_then(Value::as_str),
        "user" => payload.get("name").and_then(Value::as_str),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patterns(value: &str) -> Vec<String> {
        value.split(',').map(str::to_string).collect()
    }

    #[test]
    fn wildcard_matches_events_without_a_target() {
        assert!(autocmd_pattern_matches(Some(&patterns("*")), None));
        assert!(!autocmd_pattern_matches(Some(&patterns("*.rs")), None));
        assert!(autocmd_pattern_matches(None, None));
    }

    #[test]
    fn patterns_match_the_full_path_or_the_file_name() {
        let patterns = patterns("*.rs");
        assert!(autocmd_pattern_matches(
            Some(&patterns),
            Some("src/main.rs")
        ));
        assert!(autocmd_pattern_matches(
            Some(&patterns),
            Some("other/main.rs")
        ));
        assert!(!autocmd_pattern_matches(
            Some(&patterns),
            Some("src/main.toml")
        ));
    }

    #[test]
    fn buffer_filters_require_the_same_buffer() {
        assert!(autocmd_buffer_matches(None, Some(4)));
        assert!(autocmd_buffer_matches(Some(4), Some(4)));
        assert!(!autocmd_buffer_matches(Some(4), Some(5)));
        assert!(!autocmd_buffer_matches(Some(4), None));
    }
}
