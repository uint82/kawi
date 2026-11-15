use serde::{Deserialize, Serialize};

use super::EVENT_NAMES;

pub const MAX_PLUGIN_AUTOCMDS: usize = 256;
pub const MAX_AUTOCMD_PATTERN_BYTES: usize = 256;
pub const MAX_USER_EVENT_NAME_BYTES: usize = 64;
pub const MAX_USER_EVENT_PAYLOAD_BYTES: usize = 4_096;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutocmdSpec {
    pub event: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub buffer: Option<u64>,
    #[serde(default)]
    pub once: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisteredAutocmd {
    pub id: u64,
    pub event: String,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub buffer: Option<u64>,
    #[serde(default)]
    pub once: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AutocmdRemoveParams {
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UserEventParams {
    pub name: String,
    #[serde(default)]
    pub payload: Option<String>,
}

pub fn validate_autocmd_spec(spec: &AutocmdSpec) -> Result<(), String> {
    if !EVENT_NAMES.contains(&spec.event.as_str()) {
        return Err(format!("unknown autocmd event: {}", spec.event));
    }
    if let Some(pattern) = &spec.pattern {
        validate_autocmd_pattern(pattern)?;
    }
    Ok(())
}

pub fn validate_autocmd_pattern(pattern: &str) -> Result<(), String> {
    if pattern.is_empty() || pattern.len() > MAX_AUTOCMD_PATTERN_BYTES {
        return Err(format!(
            "autocmd pattern must be 1 to {MAX_AUTOCMD_PATTERN_BYTES} bytes"
        ));
    }
    if pattern.chars().any(char::is_control) {
        return Err("autocmd pattern contains a control character".to_string());
    }
    if pattern.split(',').any(|part| part.is_empty()) {
        return Err("autocmd pattern contains an empty entry".to_string());
    }
    Ok(())
}

pub fn validate_user_event(params: &UserEventParams) -> Result<(), String> {
    if params.name.is_empty() || params.name.len() > MAX_USER_EVENT_NAME_BYTES {
        return Err(format!(
            "user event name must be 1 to {MAX_USER_EVENT_NAME_BYTES} bytes"
        ));
    }
    if params.name.chars().any(char::is_control) {
        return Err("user event name contains a control character".to_string());
    }
    if let Some(payload) = &params.payload {
        if payload.len() > MAX_USER_EVENT_PAYLOAD_BYTES {
            return Err(format!(
                "user event payload must not exceed {MAX_USER_EVENT_PAYLOAD_BYTES} bytes"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> AutocmdSpec {
        AutocmdSpec {
            event: "file_saved".to_string(),
            pattern: Some("*.rs".to_string()),
            buffer: None,
            once: false,
        }
    }

    #[test]
    fn accepts_a_valid_autocmd_spec() {
        assert!(validate_autocmd_spec(&spec()).is_ok());
    }

    #[test]
    fn rejects_unknown_events() {
        let mut spec = spec();
        spec.event = "BufWritePost".to_string();
        assert!(validate_autocmd_spec(&spec).is_err());
    }

    #[test]
    fn rejects_invalid_patterns() {
        for pattern in ["", "*.rs,,*.toml", "a\nb"] {
            let mut spec = spec();
            spec.pattern = Some(pattern.to_string());
            assert!(validate_autocmd_spec(&spec).is_err(), "{pattern:?}");
        }
        let mut spec = spec();
        spec.pattern = Some("a".repeat(MAX_AUTOCMD_PATTERN_BYTES + 1));
        assert!(validate_autocmd_spec(&spec).is_err());
    }

    #[test]
    fn validates_user_event_names_and_payloads() {
        assert!(validate_user_event(&UserEventParams {
            name: "build-done".to_string(),
            payload: Some("ok".to_string()),
        })
        .is_ok());
        for name in ["", "bad\nname"] {
            assert!(
                validate_user_event(&UserEventParams {
                    name: name.to_string(),
                    payload: None,
                })
                .is_err(),
                "{name:?}"
            );
        }
        assert!(validate_user_event(&UserEventParams {
            name: "event".to_string(),
            payload: Some("a".repeat(MAX_USER_EVENT_PAYLOAD_BYTES + 1)),
        })
        .is_err());
    }
}
