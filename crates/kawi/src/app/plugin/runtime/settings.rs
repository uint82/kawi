use serde_json::Value;

use kawi_plugin::{PluginSetting, RpcError};

use super::PluginRuntime;

impl PluginRuntime {
    pub(crate) fn plugins_with_settings(&self) -> Vec<(String, usize)> {
        let mut plugins: Vec<_> = self
            .catalog
            .plugins
            .iter()
            .filter(|plugin| !plugin.manifest.settings.is_empty())
            .map(|plugin| (plugin.manifest.name.clone(), plugin.manifest.settings.len()))
            .collect();
        plugins.sort_by(|left, right| left.0.cmp(&right.0));
        plugins
    }

    pub(crate) fn settings_entries(
        &mut self,
        plugin: &str,
    ) -> Option<Vec<(String, PluginSetting, toml::Value)>> {
        let settings = self
            .catalog
            .plugins
            .iter()
            .find(|entry| entry.manifest.name == plugin)?
            .manifest
            .settings
            .clone();
        self.refresh_settings(plugin);
        let values = self.settings.values(plugin);
        Some(
            settings
                .into_iter()
                .map(|(key, setting)| {
                    let value = values
                        .and_then(|values| values.get(&key).cloned())
                        .unwrap_or_else(|| setting.default.clone());
                    (key, setting, value)
                })
                .collect(),
        )
    }

    pub(crate) fn set_setting(
        &mut self,
        plugin: &str,
        key: &str,
        value: toml::Value,
    ) -> Result<(), String> {
        self.settings.set_value(&self.catalog, plugin, key, value)
    }

    pub(crate) fn config_value(
        &mut self,
        plugin: &str,
        key: &str,
    ) -> Result<Option<Value>, RpcError> {
        self.refresh_settings(plugin);
        let Some(value) = self
            .settings
            .values(plugin)
            .and_then(|values| values.get(key))
        else {
            return Ok(None);
        };
        to_json(value).map(Some)
    }

    pub(crate) fn config_entries(
        &mut self,
        plugin: &str,
    ) -> Result<Vec<(String, Value)>, RpcError> {
        self.refresh_settings(plugin);
        let Some(values) = self.settings.values(plugin) else {
            return Ok(Vec::new());
        };
        values
            .iter()
            .map(|(key, value)| Ok((key.clone(), to_json(value)?)))
            .collect()
    }

    fn refresh_settings(&mut self, plugin: &str) {
        let warnings = self.settings.ensure(&self.catalog);
        for warning in warnings {
            self.push_log(plugin, &warning);
        }
    }
}

fn to_json(value: &toml::Value) -> Result<Value, RpcError> {
    serde_json::to_value(value)
        .map_err(|error| RpcError::new(kawi_plugin::INTERNAL_ERROR, error.to_string()))
}
