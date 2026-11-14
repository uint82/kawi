use kawi_plugin::{
    KeyMode, KeymapRemoveParams, KeymapSetParams, RegisteredKeymap, MAX_RUNTIME_KEYMAPS_PER_PLUGIN,
};

use super::PluginRuntime;
use crate::command::{CommandRegistry, CommandSource};
use crate::keymap::{Binding, BindingSource, KeyChord, KeyContext, KeySequence, KeymapResolver};

pub(super) struct RuntimeKeymap {
    pub(super) mode: KeyContext,
    pub(super) keys: KeySequence,
    pub(super) keys_text: String,
    pub(super) command: String,
    pub(super) description: Option<String>,
}

impl PluginRuntime {
    pub(crate) fn set_keymap(
        &mut self,
        plugin: &str,
        params: KeymapSetParams,
        commands: &CommandRegistry,
        keymaps: &mut KeymapResolver,
        leaders: &[KeyChord],
    ) -> Result<RegisteredKeymap, String> {
        if !self.contains(plugin) {
            return Err(format!("unknown plugin: {plugin}"));
        }
        let mode = key_context(params.mode);
        let keys = KeySequence::parse(&params.keys, leaders)
            .map_err(|error| format!("invalid keys: {error}"))?;
        let command = commands
            .find(&params.command)
            .ok_or_else(|| format!("unknown command: {}", params.command))?;
        self.check_keymap_owner(plugin, mode, &keys, commands, keymaps)?;
        let is_replacement = self.runtime_keymaps.get(plugin).is_some_and(|registered| {
            registered
                .iter()
                .any(|keymap| keymap.mode == mode && keymap.keys == keys)
        });
        if !is_replacement {
            let count = self.runtime_keymaps.get(plugin).map_or(0, Vec::len);
            if count >= MAX_RUNTIME_KEYMAPS_PER_PLUGIN {
                return Err(format!(
                    "at most {MAX_RUNTIME_KEYMAPS_PER_PLUGIN} runtime keymaps are allowed"
                ));
            }
        }
        let description = params.description.clone();
        keymaps.bind(
            Binding::new(command, mode, keys.clone(), BindingSource::Plugin)
                .with_description(description.clone()),
        );
        let keymap = RuntimeKeymap {
            mode,
            keys,
            keys_text: params.keys,
            command: params.command,
            description,
        };
        let result = registered_keymap(&keymap);
        let plugin_keymaps = self.runtime_keymaps.entry(plugin.to_string()).or_default();
        if let Some(existing) = plugin_keymaps
            .iter_mut()
            .find(|registered| registered.mode == mode && registered.keys == keymap.keys)
        {
            *existing = keymap;
        } else {
            plugin_keymaps.push(keymap);
        }
        Ok(result)
    }

    pub(crate) fn remove_keymap(
        &mut self,
        plugin: &str,
        params: KeymapRemoveParams,
        keymaps: &mut KeymapResolver,
        leaders: &[KeyChord],
    ) -> Result<(), String> {
        let mode = key_context(params.mode);
        let keys = KeySequence::parse(&params.keys, leaders)
            .map_err(|error| format!("invalid keys: {error}"))?;
        let Some(registered) = self.runtime_keymaps.get_mut(plugin) else {
            return Err(format!("keymap is not registered: {}", params.keys));
        };
        let Some(index) = registered
            .iter()
            .position(|keymap| keymap.mode == mode && keymap.keys == keys)
        else {
            return Err(format!("keymap is not registered: {}", params.keys));
        };
        keymaps.unbind_source(mode, &keys, BindingSource::Plugin);
        registered.remove(index);
        if registered.is_empty() {
            self.runtime_keymaps.remove(plugin);
        }
        Ok(())
    }

    pub(crate) fn registered_keymaps(&self, plugin: &str) -> Vec<RegisteredKeymap> {
        self.runtime_keymaps
            .get(plugin)
            .map(|registered| registered.iter().map(registered_keymap).collect())
            .unwrap_or_default()
    }

    pub(crate) fn clear_runtime_keymaps(&mut self, plugin: &str, keymaps: &mut KeymapResolver) {
        let Some(registered) = self.runtime_keymaps.remove(plugin) else {
            return;
        };
        for keymap in registered {
            keymaps.unbind_source(keymap.mode, &keymap.keys, BindingSource::Plugin);
        }
    }

    pub(crate) fn clear_all_runtime_keymaps(&mut self, keymaps: &mut KeymapResolver) {
        let plugins = self.runtime_keymaps.keys().cloned().collect::<Vec<_>>();
        for plugin in plugins {
            self.clear_runtime_keymaps(&plugin, keymaps);
        }
    }

    pub(super) fn remove_keymaps_for_command(
        &mut self,
        command: &super::extensions::RuntimeCommand,
        keymaps: &mut KeymapResolver,
    ) {
        let mut emptied = Vec::new();
        for (plugin, registered) in &mut self.runtime_keymaps {
            registered.retain(|keymap| {
                if keymap.command != command.name && !command.aliases.contains(&keymap.command) {
                    return true;
                }
                keymaps.unbind_source(keymap.mode, &keymap.keys, BindingSource::Plugin);
                false
            });
            if registered.is_empty() {
                emptied.push(plugin.clone());
            }
        }
        for plugin in emptied {
            self.runtime_keymaps.remove(&plugin);
        }
    }

    fn check_keymap_owner(
        &self,
        plugin: &str,
        mode: KeyContext,
        keys: &KeySequence,
        commands: &CommandRegistry,
        keymaps: &KeymapResolver,
    ) -> Result<(), String> {
        let Some(binding) = keymaps.binding_at(mode, keys.chords()) else {
            return Ok(());
        };
        match binding.source {
            BindingSource::User => Err("keymap is owned by the user".to_string()),
            BindingSource::Plugin => {
                let owner = commands
                    .spec(binding.command)
                    .and_then(|spec| match &spec.source {
                        CommandSource::Plugin(owner) => Some(owner.as_str()),
                        CommandSource::Builtin => None,
                    });
                match owner {
                    Some(owner) if owner != plugin => {
                        Err(format!("keymap is owned by plugin {owner}"))
                    }
                    _ => Ok(()),
                }
            }
            BindingSource::Default => Ok(()),
        }
    }
}

fn key_context(mode: KeyMode) -> KeyContext {
    match mode {
        KeyMode::Normal => KeyContext::Normal,
        KeyMode::Insert => KeyContext::Insert,
        KeyMode::Visual => KeyContext::Visual,
    }
}

fn key_mode(context: KeyContext) -> KeyMode {
    match context {
        KeyContext::Normal => KeyMode::Normal,
        KeyContext::Insert => KeyMode::Insert,
        KeyContext::Visual => KeyMode::Visual,
    }
}

fn registered_keymap(keymap: &RuntimeKeymap) -> RegisteredKeymap {
    RegisteredKeymap {
        mode: key_mode(keymap.mode),
        keys: keymap.keys_text.clone(),
        command: keymap.command.clone(),
        description: keymap.description.clone(),
    }
}
