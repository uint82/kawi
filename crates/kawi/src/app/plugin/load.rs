use std::path::Path;
use std::sync::mpsc;

use kawi_plugin::{
    load_catalog, load_permission_state, HostEnvironment, LoadedPlugin, PluginCatalog,
    PluginCommand, PluginHost,
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
    pub(crate) fn load(&mut self, context: PluginLoadContext<'_>, enabled: bool) -> Vec<String> {
        if !enabled {
            self.host = None;
            self.events = None;
            self.catalog = kawi_plugin::PluginCatalog::default();
            self.targets.clear();
            self.bindings.clear();
            return Vec::new();
        }
        let PluginLoadContext {
            dir,
            permissions,
            workspace,
            commands,
            keymaps,
            leaders,
            wake,
        } = context;
        self.plugins_dir = Some(dir.to_path_buf());
        self.permissions_path = Some(permissions.to_path_buf());
        self.workspace = Some(workspace.to_path_buf());
        self.wake = Some(wake.clone());
        let catalog = load_catalog(dir);
        let permission_state = load_permission_state(&catalog, permissions);
        let mut warnings = catalog
            .warnings
            .iter()
            .map(ToString::to_string)
            .chain(permission_state.warnings.iter().cloned())
            .collect::<Vec<_>>();
        self.watch_permissions(permissions, wake.clone());
        self.capabilities.replace(permission_state.grants);
        warnings.append(&mut self.reconcile_commands(&catalog, commands, keymaps, leaders));
        self.replace_host(&catalog, permissions, workspace, wake);
        self.catalog = catalog;
        self.retain_known_plugin_state();
        warnings
            .into_iter()
            .map(|warning| super::text::sanitize(&warning))
            .collect()
    }

    pub(super) fn reconcile_commands(
        &mut self,
        catalog: &PluginCatalog,
        commands: &mut CommandRegistry,
        keymaps: &mut KeymapResolver,
        leaders: &[KeyChord],
    ) -> Vec<String> {
        let mut warnings = Vec::new();
        let removed = self
            .catalog
            .plugins
            .iter()
            .map(|plugin| plugin.manifest.name.clone())
            .collect::<Vec<_>>();
        for plugin in removed {
            commands.remove_plugin(&plugin);
        }
        let previous = std::mem::take(&mut self.bindings);
        for binding in &previous {
            keymaps.unbind_source(binding.mode, &binding.keys, BindingSource::Plugin);
        }
        self.targets.clear();
        let mut bindings = Vec::new();
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
                            required: contribution
                                .capabilities
                                .iter()
                                .filter_map(|name| kawi_plugin::Capability::parse(name))
                                .collect(),
                            coalesce: contribution.coalesce_queued,
                        });
                        let (mut plugin_bindings, mut binding_warnings) =
                            plugin_bindings(plugin, contribution, id, leaders);
                        bindings.append(&mut plugin_bindings);
                        warnings.append(&mut binding_warnings);
                    }
                    Err(error) => warnings.push(format!("{}: {error}", plugin.path.display())),
                }
            }
        }
        for binding in &bindings {
            keymaps.bind(binding.clone());
        }
        self.bindings = bindings;
        warnings
    }

    pub(super) fn replace_host(
        &mut self,
        catalog: &PluginCatalog,
        permissions: &Path,
        workspace: &Path,
        wake: std::sync::mpsc::Sender<()>,
    ) {
        if catalog.plugins.is_empty() {
            self.host = None;
            self.events = None;
            return;
        }
        let (events, receiver) = mpsc::channel();
        let environment = HostEnvironment::from_paths(
            Some(workspace.to_path_buf()),
            permissions.parent().map(Path::to_path_buf),
        );
        self.host = Some(PluginHost::start(
            catalog,
            self.capabilities.clone(),
            environment,
            events,
            wake,
        ));
        self.events = Some(receiver);
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
