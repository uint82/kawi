use serde_json::Value;

use kawi_plugin::RpcError;

use super::PluginRuntime;

impl PluginRuntime {
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
