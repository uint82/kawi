use std::collections::BTreeMap;

use serde::Deserialize;

const MAX_SETTINGS: usize = 64;
const MAX_SETTING_NAME_LEN: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginSettingKind {
    Boolean,
    Integer,
    Number,
    String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginSetting {
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type")]
    pub kind: PluginSettingKind,
    pub default: toml::Value,
    #[serde(rename = "enum", default)]
    pub choices: Vec<String>,
    #[serde(default)]
    pub minimum: Option<toml::Value>,
    #[serde(default)]
    pub maximum: Option<toml::Value>,
}

impl PluginSetting {
    pub fn normalize_value(&self, value: &toml::Value) -> Result<toml::Value, String> {
        let normalized = match self.kind {
            PluginSettingKind::Boolean => match value {
                toml::Value::Boolean(value) => toml::Value::Boolean(*value),
                _ => return Err("expected a boolean".to_string()),
            },
            PluginSettingKind::Integer => match value {
                toml::Value::Integer(value) => toml::Value::Integer(*value),
                _ => return Err("expected an integer".to_string()),
            },
            PluginSettingKind::Number => match value {
                toml::Value::Float(value) if value.is_finite() => toml::Value::Float(*value),
                toml::Value::Integer(value) => toml::Value::Float(*value as f64),
                _ => return Err("expected a number".to_string()),
            },
            PluginSettingKind::String => match value {
                toml::Value::String(value) => toml::Value::String(value.clone()),
                _ => return Err("expected a string".to_string()),
            },
        };
        self.validate_choices(&normalized)?;
        self.validate_range(&normalized)?;
        Ok(normalized)
    }

    fn validate_choices(&self, value: &toml::Value) -> Result<(), String> {
        if self.choices.is_empty() {
            return Ok(());
        }
        let toml::Value::String(value) = value else {
            return Err("choices are only supported for strings".to_string());
        };
        if self.choices.iter().any(|choice| choice == value) {
            return Ok(());
        }
        Err(format!("expected one of: {}", self.choices.join(", ")))
    }

    fn validate_range(&self, value: &toml::Value) -> Result<(), String> {
        if self.minimum.is_none() && self.maximum.is_none() {
            return Ok(());
        }
        let Some(value) = numeric(value) else {
            return Err("range bounds are only supported for numbers".to_string());
        };
        if let Some(minimum) = self.minimum.as_ref().and_then(numeric) {
            if value < minimum {
                return Err(format!("expected at least {minimum}"));
            }
        }
        if let Some(maximum) = self.maximum.as_ref().and_then(numeric) {
            if value > maximum {
                return Err(format!("expected at most {maximum}"));
            }
        }
        Ok(())
    }
}

pub(super) fn validate(settings: &mut BTreeMap<String, PluginSetting>) -> Result<(), String> {
    if settings.len() > MAX_SETTINGS {
        return Err(format!("at most {MAX_SETTINGS} settings are allowed"));
    }
    for (name, setting) in settings.iter_mut() {
        validate_name(name)?;
        let default = setting
            .normalize_value(&setting.default)
            .map_err(|error| format!("setting {name}: default {error}"))?;
        validate_schema(name, setting)?;
        setting.default = default;
    }
    Ok(())
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_SETTING_NAME_LEN {
        return Err(format!(
            "setting names must be 1 to {MAX_SETTING_NAME_LEN} characters"
        ));
    }
    if !name.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '-' | '_')
    }) {
        return Err(format!("setting {name} contains an invalid character"));
    }
    Ok(())
}

fn validate_schema(name: &str, setting: &PluginSetting) -> Result<(), String> {
    validate_bounds(name, setting)?;
    if !setting.choices.is_empty() {
        if setting.kind != PluginSettingKind::String {
            return Err(format!(
                "setting {name}: choices are only supported for strings"
            ));
        }
        if setting.choices.iter().any(|choice| choice.is_empty()) {
            return Err(format!("setting {name}: choices must not be empty"));
        }
    }
    Ok(())
}

fn validate_bounds(name: &str, setting: &PluginSetting) -> Result<(), String> {
    if matches!(
        setting.kind,
        PluginSettingKind::Boolean | PluginSettingKind::String
    ) && (setting.minimum.is_some() || setting.maximum.is_some())
    {
        return Err(format!(
            "setting {name}: range bounds are only supported for numbers"
        ));
    }
    for (label, bound) in [
        ("minimum", setting.minimum.as_ref()),
        ("maximum", setting.maximum.as_ref()),
    ] {
        let Some(bound) = bound else {
            continue;
        };
        if numeric(bound).is_none() {
            return Err(format!("setting {name}: {label} must be a number"));
        }
    }
    if let (Some(minimum), Some(maximum)) = (
        setting.minimum.as_ref().and_then(numeric),
        setting.maximum.as_ref().and_then(numeric),
    ) {
        if minimum > maximum {
            return Err(format!("setting {name}: minimum exceeds maximum"));
        }
    }
    Ok(())
}

fn numeric(value: &toml::Value) -> Option<f64> {
    match value {
        toml::Value::Integer(value) => Some(*value as f64),
        toml::Value::Float(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(kind: PluginSettingKind, default: toml::Value) -> PluginSetting {
        PluginSetting {
            description: String::new(),
            kind,
            default,
            choices: Vec::new(),
            minimum: None,
            maximum: None,
        }
    }

    fn parse(text: &str) -> BTreeMap<String, PluginSetting> {
        toml::from_str(text).expect("settings")
    }

    #[test]
    fn normalizes_number_defaults_and_rejects_wrong_types() {
        let mut settings = parse(
            r#"
[count]
type = "number"
default = 2

[enabled]
type = "boolean"
default = true
"#,
        );
        validate(&mut settings).expect("valid");
        assert_eq!(settings["count"].default, toml::Value::Float(2.0));
        assert!(
            setting(PluginSettingKind::Integer, toml::Value::String("x".into()))
                .normalize_value(&toml::Value::String("x".into()))
                .is_err()
        );
    }

    #[test]
    fn enforces_choices_and_ranges() {
        let mut settings = parse(
            r#"
[mode]
type = "string"
default = "auto"
enum = ["auto", "light"]

[size]
type = "integer"
default = 10
minimum = 1
maximum = 20
"#,
        );
        validate(&mut settings).expect("valid");
        assert!(settings["mode"]
            .normalize_value(&toml::Value::String("dark".into()))
            .is_err());
        assert!(settings["size"]
            .normalize_value(&toml::Value::Integer(0))
            .is_err());
        assert!(settings["size"]
            .normalize_value(&toml::Value::Integer(21))
            .is_err());
    }

    #[test]
    fn rejects_invalid_names_bounds_and_choices() {
        for text in [
            "[Bad]\ntype = \"boolean\"\ndefault = true\n",
            "[enabled]\ntype = \"boolean\"\ndefault = true\nminimum = 0\n",
            "[mode]\ntype = \"integer\"\ndefault = 1\nenum = [\"one\"]\n",
            "[size]\ntype = \"integer\"\ndefault = 5\nminimum = 10\nmaximum = 1\n",
            "[size]\ntype = \"integer\"\ndefault = 5\nminimum = \"low\"\n",
        ] {
            let mut settings = parse(text);
            assert!(validate(&mut settings).is_err(), "{text}");
        }
    }
}
