use serde_json::Value;

use crate::protocol::{ConfigSetting, EditorCommandInfo, EditorListCommandsResult, RpcError};

use super::super::bindings::kawi::plugin::types::{CommandInfo, Setting, SettingValue};

pub fn command_infos_from_json(value: Value) -> Result<Vec<CommandInfo>, RpcError> {
    let result: EditorListCommandsResult = serde_json::from_value(value)
        .map_err(|error| RpcError::invalid_params(error.to_string()))?;
    Ok(result.commands.into_iter().map(command_info).collect())
}

fn command_info(info: EditorCommandInfo) -> CommandInfo {
    CommandInfo {
        id: info.id,
        description: info.description,
        aliases: info.aliases,
    }
}

pub fn setting_value_from_json(value: Value) -> Result<SettingValue, RpcError> {
    match value {
        Value::Bool(value) => Ok(SettingValue::Boolean(value)),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                return Ok(SettingValue::Integer(value));
            }
            let value = value
                .as_f64()
                .filter(|value| value.is_finite())
                .ok_or_else(|| RpcError::invalid_params("settings must be finite numbers"))?;
            Ok(SettingValue::Float(value))
        }
        Value::String(value) => Ok(SettingValue::Text(value)),
        _ => Err(RpcError::invalid_params(
            "settings must be boolean, integer, float, or text",
        )),
    }
}

pub fn settings_from_json(settings: Vec<ConfigSetting>) -> Result<Vec<Setting>, RpcError> {
    settings
        .into_iter()
        .map(|setting| {
            Ok(Setting {
                key: setting.key,
                value: setting_value_from_json(setting.value)?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_command_infos() {
        let infos = command_infos_from_json(serde_json::json!({
            "commands": [{
                "id": "file.save",
                "description": "save the buffer",
                "aliases": ["save"],
            }],
        }))
        .expect("command infos");
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].id, "file.save");
        assert_eq!(infos[0].description, "save the buffer");
        assert_eq!(infos[0].aliases, ["save"]);
    }

    #[test]
    fn converts_setting_values_and_lists() {
        let value = setting_value_from_json(serde_json::json!(3)).expect("integer");
        assert!(matches!(value, SettingValue::Integer(3)));
        let value = setting_value_from_json(serde_json::json!("dark")).expect("text");
        assert!(matches!(value, SettingValue::Text(ref value) if value == "dark"));
        let settings = settings_from_json(vec![ConfigSetting {
            key: "size".to_string(),
            value: serde_json::json!(4),
        }])
        .expect("settings");
        assert_eq!(settings.len(), 1);
        assert_eq!(settings[0].key, "size");
        assert!(matches!(settings[0].value, SettingValue::Integer(4)));
    }

    #[test]
    fn rejects_non_scalar_setting_values() {
        for value in [
            serde_json::json!(null),
            serde_json::json!([1]),
            serde_json::json!({"a": 1}),
        ] {
            assert!(setting_value_from_json(value).is_err());
        }
    }
}
