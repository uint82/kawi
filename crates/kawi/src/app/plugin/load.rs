use std::path::Path;
use std::sync::mpsc;

use kawi_plugin::{
    load_catalog, load_permission_state, HostEnvironment, LoadedPlugin, PluginCommand, PluginHost,
};

use super::{PluginCommandTarget, PluginRuntime};
use crate::command::{CommandId, CommandRegistry};
use crate::keymap::KeyChord;
use crate::keymap::{has_reserved_prefix, Binding, BindingSource, KeyContext, KeymapResolver};

pub(crate) struct PluginLoadContext<'a> {
    pub(crate) dir: &'a Path,
    pub(crate) permissions: &'a Path,
    pub(crate) workspace: &'a Path,
    pub(crate) commands: &'a mut CommandRegistry,
    pub(crate) keymaps: &'a mut KeymapResolver,
    pub(crate) leaders: &'a [KeyChord],
    pub(crate) wake: std::sync::mpsc::Sender<()>,
}

impl PluginRuntime {
    pub(crate) fn load(&mut self, context: PluginLoadContext<'_>) -> Vec<String> {
        let PluginLoadContext {
            dir,
            permissions,
            workspace,
            commands,
            keymaps,
            leaders,
            wake,
        } = context;
        let catalog = load_catalog(dir);
        let permission_state = load_permission_state(&catalog, permissions);
        let mut warnings = catalog
            .warnings
            .iter()
            .map(ToString::to_string)
            .chain(permission_state.warnings.iter().cloned())
            .map(|warning| super::text::sanitize(&warning))
            .collect::<Vec<_>>();
        self.permissions_path = Some(permissions.to_path_buf());
        self.watch_permissions(permissions, wake.clone());
        self.capabilities.replace(permission_state.grants);
        let mut pending_bindings = Vec::new();
        for plugin in &catalog.plugins {
            for contribution in &plugin.manifest.commands {
                let name = plugin.command_name(contribution);
                let target = self.targets.len() as u32;
                let description = command_description(contribution);
                match commands.register_plugin(
                    name.clone(),
                    description.clone(),
                    plugin.manifest.name.clone(),
                    target,
                ) {
                    Ok(id) => {
                        self.targets.push(PluginCommandTarget {
                            plugin: plugin.manifest.name.clone(),
                            command: contribution.id.clone(),
                        });
                        let (mut bindings, mut binding_warnings) =
                            plugin_bindings(plugin, contribution, id, leaders);
                        pending_bindings.append(&mut bindings);
                        warnings.append(&mut binding_warnings);
                    }
                    Err(error) => warnings.push(format!("{}: {error}", plugin.path.display())),
                }
            }
        }
        for binding in &pending_bindings {
            keymaps.bind(binding.clone());
        }
        self.bindings = pending_bindings;
        if !catalog.plugins.is_empty() {
            let (events, receiver) = mpsc::channel();
            let environment = HostEnvironment::from_paths(
                Some(workspace.to_path_buf()),
                permissions.parent().map(Path::to_path_buf),
            );
            self.host = Some(PluginHost::start(
                &catalog,
                self.capabilities.clone(),
                environment,
                events,
                wake,
            ));
            self.events = Some(receiver);
        }
        self.catalog = catalog;
        self.retain_known_plugin_state();
        warnings
            .into_iter()
            .map(|warning| super::text::sanitize(&warning))
            .collect()
    }
}

fn command_description(contribution: &PluginCommand) -> String {
    let text = if contribution.description.trim().is_empty() {
        &contribution.title
    } else {
        &contribution.description
    };
    super::text::sanitize(text)
}

fn plugin_bindings(
    plugin: &LoadedPlugin,
    contribution: &PluginCommand,
    command: CommandId,
    leaders: &[KeyChord],
) -> (Vec<Binding>, Vec<String>) {
    let mut bindings = Vec::new();
    let mut warnings = Vec::new();
    let description = command_description(contribution);
    for (mode, sequences) in &contribution.keybindings {
        let Some(context) = KeyContext::parse(mode) else {
            warnings.push(format!(
                "{}: unknown keybinding mode: {mode}",
                plugin.path.display()
            ));
            continue;
        };
        for sequence in sequences {
            match crate::keymap::config::expand_sequences(sequence, leaders) {
                Ok(expanded) => {
                    for keys in expanded {
                        if has_reserved_prefix(&keys) {
                            warnings.push(format!(
                                "{}: count and register prefixes are reserved: {sequence}",
                                plugin.path.display()
                            ));
                            continue;
                        }
                        bindings.push(
                            Binding::new(command, context, keys, BindingSource::Plugin)
                                .with_description(Some(description.clone())),
                        );
                    }
                }
                Err(error) => warnings.push(format!(
                    "{}: invalid keybinding {sequence}: {error}",
                    plugin.path.display()
                )),
            }
        }
    }
    (bindings, warnings)
}
