use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    MAX_BUFFER_VARIABLES, MAX_GLOBAL_VARIABLES, MAX_PLUGIN_VARIABLES, MAX_VARIABLE_KEY_BYTES,
    MAX_VARIABLE_VALUE_BYTES, MAX_WINDOW_VARIABLES,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VariableScope {
    Global,
    Plugin,
    Buffer,
    Window,
}

impl VariableScope {
    pub fn name(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Plugin => "plugin",
            Self::Buffer => "buffer",
            Self::Window => "window",
        }
    }

    pub fn limit(self) -> usize {
        match self {
            Self::Global => MAX_GLOBAL_VARIABLES,
            Self::Plugin => MAX_PLUGIN_VARIABLES,
            Self::Buffer => MAX_BUFFER_VARIABLES,
            Self::Window => MAX_WINDOW_VARIABLES,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableTarget {
    pub scope: VariableScope,
    #[serde(default)]
    pub buffer: Option<u64>,
    #[serde(default)]
    pub window: Option<u64>,
}

impl VariableTarget {
    pub fn validate(self) -> Result<Self, String> {
        match self.scope {
            VariableScope::Global | VariableScope::Plugin => {
                if self.buffer.is_some() || self.window.is_some() {
                    return Err(format!(
                        "{} variables take no buffer or window",
                        self.scope.name()
                    ));
                }
            }
            VariableScope::Buffer => {
                if self.buffer.is_none() {
                    return Err("buffer variables require a buffer".to_string());
                }
                if self.window.is_some() {
                    return Err("buffer variables take no window".to_string());
                }
            }
            VariableScope::Window => {
                if self.window.is_none() {
                    return Err("window variables require a window".to_string());
                }
                if self.buffer.is_some() {
                    return Err("window variables take no buffer".to_string());
                }
            }
        }
        Ok(self)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableTargetParams {
    pub target: VariableTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableParams {
    pub target: VariableTarget,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariableSetParams {
    pub target: VariableTarget,
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Variable {
    pub key: String,
    pub value: Value,
}

pub fn validate_variable_key(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Err("variable key must not be empty".to_string());
    }
    if key.len() > MAX_VARIABLE_KEY_BYTES {
        return Err(format!(
            "variable key exceeds {MAX_VARIABLE_KEY_BYTES} bytes"
        ));
    }
    Ok(())
}

pub fn validate_variable_value(value: &Value) -> Result<(), String> {
    if !matches!(value, Value::Bool(_) | Value::Number(_) | Value::String(_)) {
        return Err("variable values must be boolean, integer, float, or text".to_string());
    }
    let size = serde_json::to_vec(value)
        .map_err(|error| format!("invalid variable value: {error}"))?
        .len();
    if size > MAX_VARIABLE_VALUE_BYTES {
        return Err(format!(
            "variable value exceeds {MAX_VARIABLE_VALUE_BYTES} bytes"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn scopes_validate_their_required_and_forbidden_ids() {
        for target in [
            VariableTarget {
                scope: VariableScope::Global,
                buffer: None,
                window: None,
            },
            VariableTarget {
                scope: VariableScope::Plugin,
                buffer: None,
                window: None,
            },
            VariableTarget {
                scope: VariableScope::Buffer,
                buffer: Some(1),
                window: None,
            },
            VariableTarget {
                scope: VariableScope::Window,
                buffer: None,
                window: Some(1),
            },
        ] {
            assert!(target.validate().is_ok());
        }
        for target in [
            VariableTarget {
                scope: VariableScope::Global,
                buffer: Some(1),
                window: None,
            },
            VariableTarget {
                scope: VariableScope::Plugin,
                buffer: None,
                window: Some(1),
            },
            VariableTarget {
                scope: VariableScope::Buffer,
                buffer: None,
                window: None,
            },
            VariableTarget {
                scope: VariableScope::Buffer,
                buffer: Some(1),
                window: Some(1),
            },
            VariableTarget {
                scope: VariableScope::Window,
                buffer: None,
                window: None,
            },
            VariableTarget {
                scope: VariableScope::Window,
                buffer: Some(1),
                window: Some(1),
            },
        ] {
            assert!(target.validate().is_err());
        }
    }

    #[test]
    fn keys_are_non_empty_and_bounded() {
        assert!(validate_variable_key("").is_err());
        assert!(validate_variable_key(&"k".repeat(MAX_VARIABLE_KEY_BYTES)).is_ok());
        assert!(validate_variable_key(&"k".repeat(MAX_VARIABLE_KEY_BYTES + 1)).is_err());
    }

    #[test]
    fn values_are_bounded_scalars() {
        for value in [json!(true), json!(1), json!(-2.5), json!("text")] {
            assert!(validate_variable_value(&value).is_ok(), "{value}");
        }
        for value in [json!(null), json!([1]), json!({ "a": 1 })] {
            assert!(validate_variable_value(&value).is_err(), "{value}");
        }
        assert!(validate_variable_value(&json!("x".repeat(MAX_VARIABLE_VALUE_BYTES))).is_err());
    }

    #[test]
    fn scope_limits_match_the_protocol_constants() {
        assert_eq!(VariableScope::Global.limit(), MAX_GLOBAL_VARIABLES);
        assert_eq!(VariableScope::Plugin.limit(), MAX_PLUGIN_VARIABLES);
        assert_eq!(VariableScope::Buffer.limit(), MAX_BUFFER_VARIABLES);
        assert_eq!(VariableScope::Window.limit(), MAX_WINDOW_VARIABLES);
        for scope in [
            VariableScope::Global,
            VariableScope::Plugin,
            VariableScope::Buffer,
            VariableScope::Window,
        ] {
            assert_eq!(
                VariableScope::name(scope),
                serde_json::to_value(scope).unwrap()
            );
        }
    }
}
