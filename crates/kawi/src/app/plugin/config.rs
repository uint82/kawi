use serde_json::Value;

use kawi_plugin::{
    ConfigGetParams, ConfigGetResult, ConfigListResult, ConfigSetting, RpcError, INTERNAL_ERROR,
    INVALID_PARAMS,
};

use super::super::App;
use super::host_methods::invalid_params;

const MAX_SETTING_NAME_LEN: usize = 64;

impl App {
    pub(crate) fn plugin_config_get(
        &mut self,
        plugin: &str,
        params: Value,
    ) -> Result<Value, RpcError> {
        let params: ConfigGetParams = serde_json::from_value(params).map_err(invalid_params)?;
        validate_setting_key(&params.key)?;
        let value = self.plugins.config_value(plugin, &params.key)?;
        serialize(ConfigGetResult { value })
    }

    pub(crate) fn plugin_config_list(
        &mut self,
        plugin: &str,
        _params: Value,
    ) -> Result<Value, RpcError> {
        let settings = self
            .plugins
            .config_entries(plugin)?
            .into_iter()
            .map(|(key, value)| ConfigSetting { key, value })
            .collect();
        serialize(ConfigListResult { settings })
    }
}

fn validate_setting_key(key: &str) -> Result<(), RpcError> {
    if key.is_empty() || key.len() > MAX_SETTING_NAME_LEN {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("setting keys must be 1 to {MAX_SETTING_NAME_LEN} characters"),
        ));
    }
    if !key.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '-' | '_')
    }) {
        return Err(RpcError::new(
            INVALID_PARAMS,
            format!("setting key {key} contains an invalid character"),
        ));
    }
    Ok(())
}

fn serialize<T: serde::Serialize>(value: T) -> Result<Value, RpcError> {
    serde_json::to_value(value).map_err(|error| RpcError::new(INTERNAL_ERROR, error.to_string()))
}
