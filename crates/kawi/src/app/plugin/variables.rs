use std::collections::BTreeMap;

use kawi_plugin::{validate_variable_key, VariableScope, VariableTarget, VariableTargetParams};
use serde_json::Value;

use super::runtime::PluginRuntime;

impl PluginRuntime {
    pub(super) fn get_variable(
        &self,
        plugin: &str,
        target: VariableTarget,
        key: &str,
    ) -> Option<Value> {
        self.variable_map(plugin, target)?.get(key).cloned()
    }

    pub(super) fn set_variable(
        &mut self,
        plugin: &str,
        target: VariableTarget,
        key: &str,
        value: Value,
    ) -> Result<(), String> {
        validate_variable_key(key)?;
        let limit = target.scope.limit();
        let map = self
            .variable_map_mut(plugin, target)
            .ok_or_else(|| format!("{} variable target is missing", target.scope.name()))?;
        if !map.contains_key(key) && map.len() >= limit {
            return Err(format!(
                "{} variable limit reached: {limit}",
                target.scope.name()
            ));
        }
        map.insert(key.to_string(), value);
        Ok(())
    }

    pub(super) fn delete_variable(
        &mut self,
        plugin: &str,
        target: VariableTarget,
        key: &str,
    ) -> bool {
        self.variable_map_mut(plugin, target)
            .is_some_and(|variables| variables.remove(key).is_some())
    }

    pub(super) fn list_variables(
        &self,
        plugin: &str,
        target: VariableTargetParams,
    ) -> Vec<kawi_plugin::Variable> {
        self.variable_map(plugin, target.target)
            .map(|variables| {
                variables
                    .iter()
                    .map(|(key, value)| kawi_plugin::Variable {
                        key: key.clone(),
                        value: value.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn variable_map(
        &self,
        plugin: &str,
        target: VariableTarget,
    ) -> Option<&BTreeMap<String, Value>> {
        match target.scope {
            VariableScope::Global => Some(&self.global_variables),
            VariableScope::Plugin => self.plugin_variables.get(plugin),
            VariableScope::Buffer => self.buffer_variables.get(&target.buffer?),
            VariableScope::Window => self.window_variables.get(&target.window?),
        }
    }

    fn variable_map_mut(
        &mut self,
        plugin: &str,
        target: VariableTarget,
    ) -> Option<&mut BTreeMap<String, Value>> {
        match target.scope {
            VariableScope::Global => Some(&mut self.global_variables),
            VariableScope::Plugin => {
                Some(self.plugin_variables.entry(plugin.to_string()).or_default())
            }
            VariableScope::Buffer => target
                .buffer
                .map(|buffer| self.buffer_variables.entry(buffer).or_default()),
            VariableScope::Window => target
                .window
                .map(|window| self.window_variables.entry(window).or_default()),
        }
    }
}
