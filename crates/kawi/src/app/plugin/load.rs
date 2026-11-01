use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::mpsc;

use kawi_plugin::{
    load_catalog, load_managed_catalog, EffectiveCapabilities, HostEnvironment, LoadedPlugin,
    PluginCatalog, PluginCommand, PluginHost, PluginLock,
};

use super::{PluginCommandTarget, PluginRuntime};
use crate::command::{CommandId, CommandRegistry};
use crate::keymap::KeyChord;
use crate::keymap::{has_reserved_prefix, Binding, BindingSource, KeyContext, KeymapResolver};

pub(crate) struct PluginLoadContext<'a> {
    pub(crate) installed: &'a Path,
    pub(crate) dev: &'a Path,
    pub(crate) lock: &'a PluginLock,
    pub(crate) workspace: &'a Path,
    pub(crate) settings: &'a Path,
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
            self.catalog = PluginCatalog::default();
            self.targets.clear();
            self.bindings.clear();
            self.settings.set_path(context.settings.to_path_buf());
            return Vec::new();
        }
        let PluginLoadContext {
            installed,
            dev,
            lock,
            workspace,
            settings,
            commands,
            keymaps,
            leaders,
            wake,
        } = context;
        self.installed_dir = Some(installed.to_path_buf());
        self.settings.set_path(settings.to_path_buf());
        self.dev_dir = Some(dev.to_path_buf());
        self.lock = lock.clone();
        self.workspace = Some(workspace.to_path_buf());
        self.wake = Some(wake.clone());
        let (catalog, dev_plugins, mut warnings) = load_catalogs(installed, dev, lock, true);
        self.dev_plugins = dev_plugins;
        self.capabilities.replace(capability_grants(&catalog));
        warnings.append(&mut self.reconcile_commands(&catalog, commands, keymaps, leaders));
        self.replace_host(&catalog, workspace, wake);
        self.catalog = catalog;
        warnings.append(&mut self.settings.reload(&self.catalog));
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
        workspace: &Path,
        wake: std::sync::mpsc::Sender<()>,
    ) {
        if catalog.plugins.is_empty() {
            self.host = None;
            self.events = None;
            return;
        }
        let (events, receiver) = mpsc::channel();
        let config = self.dev_dir.as_deref().and_then(Path::parent);
        let environment = HostEnvironment::from_paths(
            Some(workspace.to_path_buf()),
            config.map(Path::to_path_buf),
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

pub(super) fn load_catalogs(
    installed: &Path,
    dev: &Path,
    lock: &PluginLock,
    warn_dev: bool,
) -> (PluginCatalog, BTreeSet<String>, Vec<String>) {
    let mut catalog = load_managed_catalog(installed, lock);
    let mut warnings = catalog
        .warnings
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let managed_names = lock.plugins.keys().cloned().collect::<BTreeSet<_>>();
    let mut dev_catalog = load_catalog(dev);
    warnings.extend(dev_catalog.warnings.iter().map(ToString::to_string));
    let mut dev_names = BTreeSet::new();
    for plugin in dev_catalog.plugins.drain(..) {
        if managed_names.contains(&plugin.manifest.name)
            || !dev_names.insert(plugin.manifest.name.clone())
        {
            warnings.push(format!(
                "{}: duplicate development plugin {}",
                plugin.path.display(),
                plugin.manifest.name
            ));
            continue;
        }
        if warn_dev {
            warnings.push(format!(
                "{}: development plugin loaded from {}; use :plugin install for managed plugins",
                plugin.path.display(),
                dev.display()
            ));
        }
        catalog.plugins.push(plugin);
    }
    catalog.warnings.clear();
    (catalog, dev_names, warnings)
}

pub(super) fn capability_grants(
    catalog: &PluginCatalog,
) -> BTreeMap<String, EffectiveCapabilities> {
    catalog
        .plugins
        .iter()
        .map(|plugin| {
            (
                plugin.manifest.name.clone(),
                plugin.manifest.capability_requests.granted(),
            )
        })
        .collect()
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
