use kawi_plugin::{load_catalog, load_permission_state, PluginCatalog, PluginError};

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
        let (Some(dir), Some(permissions), Some(workspace), Some(wake)) = (
            self.plugins_dir.clone(),
            self.permissions_path.clone(),
            self.workspace.clone(),
            self.wake.clone(),
        ) else {
            return Ok(Vec::new());
        };
        let loaded = load_catalog(&dir);
        let catalog = merge_catalog(&self.catalog, loaded, plugin)?;
        let permission_state = load_permission_state(&catalog, &permissions);
        let mut warnings = catalog
            .warnings
            .iter()
            .map(ToString::to_string)
            .chain(permission_state.warnings.iter().cloned())
            .collect::<Vec<_>>();
        self.capabilities.replace(permission_state.grants);
        warnings.append(&mut self.reconcile_commands(&catalog, commands, keymaps, leaders));
        let context = ReloadContext {
            permissions: &permissions,
            workspace: &workspace,
            wake,
        };
        self.reload_host(&catalog, plugin, context)?;
        self.catalog = catalog;
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
        context: ReloadContext<'_>,
    ) -> Result<(), PluginError> {
        let Some(host) = self.host.as_mut() else {
            self.replace_host(
                catalog,
                context.permissions,
                context.workspace,
                context.wake,
            );
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

struct ReloadContext<'a> {
    permissions: &'a std::path::Path,
    workspace: &'a std::path::Path,
    wake: std::sync::mpsc::Sender<()>,
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
