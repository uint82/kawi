use kawi_plugin::{PluginCatalog, PluginError};

use super::load::{capability_grants, load_catalogs};
use super::PluginRuntime;
use crate::command::CommandRegistry;
use crate::keymap::KeyChord;
use crate::keymap::KeymapResolver;

impl PluginRuntime {
    pub(crate) fn reload(
        &mut self,
        commands: &mut CommandRegistry,
        keymaps: &mut KeymapResolver,
        leaders: &[KeyChord],
        plugin: Option<&str>,
    ) -> Result<Vec<String>, PluginError> {
        let (Some(installed), Some(dev), Some(workspace), Some(wake)) = (
            self.installed_dir.clone(),
            self.dev_dir.clone(),
            self.workspace.clone(),
            self.wake.clone(),
        ) else {
            return Ok(Vec::new());
        };
        self.drop_queues(plugin);
        let (loaded, dev_plugins, mut warnings) =
            load_catalogs(&installed, &dev, &self.lock, false);
        let catalog = merge_catalog(&self.catalog, loaded, plugin)?;
        self.dev_plugins = dev_plugins;
        self.capabilities.replace(capability_grants(&catalog));
        warnings.append(&mut self.reconcile_commands(&catalog, commands, keymaps, leaders));
        self.reload_host(&catalog, plugin, &workspace, wake)?;
        self.catalog = catalog;
        warnings.append(&mut self.settings.reload(&self.catalog));
        self.reset_message_throttle(plugin);
        self.retain_known_plugin_state();
        Ok(warnings
            .into_iter()
            .map(|warning| super::text::sanitize(&warning))
            .collect())
    }

    fn reset_message_throttle(&mut self, plugin: Option<&str>) {
        match plugin {
            Some(plugin) => {
                self.last_message.remove(plugin);
                self.throttled_messages.remove(plugin);
            }
            None => {
                self.last_message.clear();
                self.throttled_messages.clear();
            }
        }
    }

    fn reload_host(
        &mut self,
        catalog: &PluginCatalog,
        plugin: Option<&str>,
        workspace: &std::path::Path,
        wake: std::sync::mpsc::Sender<()>,
    ) -> Result<(), PluginError> {
        let Some(host) = self.host.as_mut() else {
            self.replace_host(catalog, workspace, wake);
            return Ok(());
        };
        let target = plugin.filter(|name| {
            catalog
                .plugins
                .iter()
                .any(|plugin| plugin.manifest.name == *name)
        });
        host.reload(catalog, target)
    }
}

fn merge_catalog(
    current: &PluginCatalog,
    loaded: PluginCatalog,
    plugin: Option<&str>,
) -> Result<PluginCatalog, PluginError> {
    let Some(name) = plugin else {
        return Ok(loaded);
    };
    let next = loaded
        .plugins
        .iter()
        .find(|plugin| plugin.manifest.name == name)
        .cloned();
    let existing = current
        .plugins
        .iter()
        .position(|plugin| plugin.manifest.name == name);
    if existing.is_none() && next.is_none() {
        return Err(PluginError::Spawn(format!("unknown plugin: {name}")));
    }
    let mut plugins = current.plugins.clone();
    match (existing, next) {
        (Some(index), Some(plugin)) => plugins[index] = plugin,
        (Some(index), None) => {
            plugins.remove(index);
        }
        (None, Some(plugin)) => plugins.push(plugin),
        (None, None) => {}
    }
    Ok(PluginCatalog {
        plugins,
        warnings: loaded.warnings,
    })
}
