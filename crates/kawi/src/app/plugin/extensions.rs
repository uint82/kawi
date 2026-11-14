use kawi_plugin::{CommandRegisterParams, RegisteredCommand, MAX_RUNTIME_COMMANDS_PER_PLUGIN};

use super::runtime::PluginCommandTarget;
use super::PluginRuntime;
use crate::command::{CommandRegistry, PluginCommandRegistration};
use crate::keymap::KeymapResolver;

#[derive(Clone)]
pub(super) struct RuntimeCommand {
    pub(super) name: String,
    pub(super) title: String,
    pub(super) description: String,
    pub(super) aliases: Vec<String>,
}

impl PluginRuntime {
    pub(crate) fn register_command(
        &mut self,
        plugin: &str,
        params: CommandRegisterParams,
        commands: &mut CommandRegistry,
    ) -> Result<RegisteredCommand, String> {
        if !self.contains(plugin) {
            return Err(format!("unknown plugin: {plugin}"));
        }
        let registered = self.runtime_commands.entry(plugin.to_string()).or_default();
        if registered.contains_key(&params.id) {
            return Err(format!("command already registered: {}", params.id));
        }
        if registered.len() >= MAX_RUNTIME_COMMANDS_PER_PLUGIN {
            return Err(format!(
                "at most {MAX_RUNTIME_COMMANDS_PER_PLUGIN} runtime commands are allowed"
            ));
        }
        let name = format!("{plugin}.{}", params.id);
        let description = command_description(&params);
        let target = self.targets.len() as u32;
        commands.register_plugin(PluginCommandRegistration {
            name: name.clone(),
            aliases: params.aliases.clone(),
            description: description.clone(),
            plugin: plugin.to_string(),
            target,
        })?;
        self.targets.push(PluginCommandTarget {
            plugin: plugin.to_string(),
            command: params.id.clone(),
            required: Vec::new(),
            coalesce: false,
            accepts_args: true,
        });
        registered.insert(
            params.id.clone(),
            RuntimeCommand {
                name: name.clone(),
                title: params.title.clone(),
                description: description.clone(),
                aliases: params.aliases.clone(),
            },
        );
        Ok(RegisteredCommand {
            id: params.id,
            name,
            title: params.title,
            description,
            aliases: params.aliases,
        })
    }

    pub(crate) fn unregister_command(
        &mut self,
        plugin: &str,
        id: &str,
        commands: &mut CommandRegistry,
        keymaps: &mut KeymapResolver,
    ) -> Result<(), String> {
        let command = self
            .runtime_commands
            .get(plugin)
            .and_then(|registered| registered.get(id))
            .cloned()
            .ok_or_else(|| format!("command is not registered: {id}"))?;
        commands.unregister_plugin(&command.name, plugin)?;
        self.remove_keymaps_for_command(&command, keymaps);
        if let Some(registered) = self.runtime_commands.get_mut(plugin) {
            registered.remove(id);
            if registered.is_empty() {
                self.runtime_commands.remove(plugin);
            }
        }
        Ok(())
    }

    pub(crate) fn registered_commands(&self, plugin: &str) -> Vec<RegisteredCommand> {
        self.runtime_commands
            .get(plugin)
            .map(|registered| {
                registered
                    .iter()
                    .map(|(id, command)| RegisteredCommand {
                        id: id.clone(),
                        name: command.name.clone(),
                        title: command.title.clone(),
                        description: command.description.clone(),
                        aliases: command.aliases.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn clear_runtime_commands(
        &mut self,
        plugin: &str,
        commands: &mut CommandRegistry,
        keymaps: &mut KeymapResolver,
    ) {
        let Some(registered) = self.runtime_commands.remove(plugin) else {
            return;
        };
        for command in registered.into_values() {
            let _ = commands.unregister_plugin(&command.name, plugin);
            self.remove_keymaps_for_command(&command, keymaps);
        }
    }

    pub(crate) fn clear_all_runtime_commands(
        &mut self,
        commands: &mut CommandRegistry,
        keymaps: &mut KeymapResolver,
    ) {
        let plugins = self.runtime_commands.keys().cloned().collect::<Vec<_>>();
        for plugin in plugins {
            self.clear_runtime_commands(&plugin, commands, keymaps);
        }
    }

    pub(crate) fn target_accepts_args(&self, target: u32) -> bool {
        self.targets
            .get(target as usize)
            .is_some_and(|target| target.accepts_args)
    }
}

fn command_description(params: &CommandRegisterParams) -> String {
    params
        .description
        .as_deref()
        .filter(|description| !description.trim().is_empty())
        .unwrap_or(&params.title)
        .to_string()
}
