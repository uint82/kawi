use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::capability::parse_capability_requests;

use super::validation;
use super::{
    LoadedPlugin, PluginCatalog, PluginManifest, PluginWarning, MANIFEST_FILE, PERMISSIONS_FILE,
};

pub fn load_catalog(dir: &Path) -> PluginCatalog {
    let mut catalog = PluginCatalog::default();
    let entries = match read_entries(dir) {
        Ok(entries) => entries,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                catalog.warnings.push(PluginWarning {
                    path: dir.to_path_buf(),
                    message: format!("cannot read plugin directory: {error}"),
                });
            }
            return catalog;
        }
    };
    let mut names = HashSet::new();
    for entry in entries {
        let Some(plugin) = load_plugin(&entry, &mut catalog.warnings) else {
            continue;
        };
        if !names.insert(plugin.manifest.name.clone()) {
            catalog.warnings.push(PluginWarning {
                path: plugin.path.clone(),
                message: format!("duplicate plugin name: {}", plugin.manifest.name),
            });
            continue;
        }
        catalog.plugins.push(plugin);
    }
    catalog
}

fn read_entries(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    entries.sort();
    Ok(entries)
}

fn load_plugin(dir: &Path, warnings: &mut Vec<PluginWarning>) -> Option<LoadedPlugin> {
    let path = dir.join(MANIFEST_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) => {
            if error.kind() != std::io::ErrorKind::NotFound {
                warnings.push(PluginWarning {
                    path: path.clone(),
                    message: format!("cannot read manifest: {error}"),
                });
            }
            return None;
        }
    };
    let mut manifest = match toml::from_str::<PluginManifest>(&text) {
        Ok(manifest) => manifest,
        Err(error) => {
            warnings.push(PluginWarning {
                path: path.clone(),
                message: error.to_string(),
            });
            return None;
        }
    };
    if dir.join(PERMISSIONS_FILE).is_file() {
        warnings.push(PluginWarning {
            path: dir.join(PERMISSIONS_FILE),
            message: format!(
                "plugin directory contains {PERMISSIONS_FILE}; only the user config file is read"
            ),
        });
    }
    match validation::validate_manifest(&mut manifest, dir) {
        Ok(artifact) => {
            if let Some(reason) = &artifact.disabled_reason {
                warnings.push(PluginWarning {
                    path: path.clone(),
                    message: reason.clone(),
                });
            }
            let (mut requests, capability_warnings) =
                parse_capability_requests(&manifest.capabilities);
            let gates = match validation::command_capabilities(&manifest.commands) {
                Ok(gates) => gates,
                Err(message) => {
                    warnings.push(PluginWarning {
                        path: path.clone(),
                        message,
                    });
                    return None;
                }
            };
            for (id, capabilities) in &gates {
                if let Some(capability) = capabilities
                    .iter()
                    .find(|capability| !requests.contains(**capability))
                {
                    warnings.push(PluginWarning {
                        path: path.clone(),
                        message: format!(
                            "command {id} requires {}, which the plugin does not request",
                            capability.name()
                        ),
                    });
                    return None;
                }
            }
            requests.set_command_gates(gates);
            if let Some(hash) = artifact.computed_hash.as_deref() {
                requests.set_artifact_hash(hash);
            }
            manifest.capability_requests = requests;
            for warning in capability_warnings {
                warnings.push(PluginWarning {
                    path: path.clone(),
                    message: format!("capabilities: {warning}"),
                });
            }
            Some(LoadedPlugin {
                manifest,
                dir: dir.to_path_buf(),
                path,
                computed_hash: artifact.computed_hash,
                disabled_reason: artifact.disabled_reason,
            })
        }
        Err(message) => {
            warnings.push(PluginWarning {
                path: path.clone(),
                message,
            });
            None
        }
    }
}
