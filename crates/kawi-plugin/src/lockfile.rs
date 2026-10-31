use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::manifest::{load_catalog, PluginCatalog, PluginWarning};

const LOCK_VERSION: u64 = 1;
const TEMP_SUFFIX: &str = ".tmp";
const MISSING_LOCK_ENTRY: &str = "plugin is not listed in the lockfile; ignoring";
const HASH_MISMATCH: &str = "component hash does not match the lockfile; ignoring";
const API_MISMATCH: &str = "plugin API does not match the lockfile; ignoring";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PluginLock {
    #[serde(default = "lock_version")]
    pub version: u64,
    #[serde(default)]
    pub plugins: BTreeMap<String, LockedPlugin>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct LockedPlugin {
    pub source: String,
    #[serde(default)]
    pub reference: Option<String>,
    pub commit: String,
    pub sha256: String,
    pub api: u64,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl Default for PluginLock {
    fn default() -> Self {
        Self {
            version: LOCK_VERSION,
            plugins: BTreeMap::new(),
        }
    }
}

impl PluginLock {
    pub fn load(path: &Path) -> Result<Self, String> {
        match fs::read_to_string(path) {
            Ok(text) => Self::parse(path, &text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(format!("cannot read {}: {error}", path.display())),
        }
    }

    fn parse(path: &Path, text: &str) -> Result<Self, String> {
        let lock: Self = toml::from_str(text)
            .map_err(|error| format!("cannot parse {}: {error}", path.display()))?;
        if lock.version != LOCK_VERSION {
            return Err(format!(
                "unsupported lockfile version {} in {}, expected {LOCK_VERSION}",
                lock.version,
                path.display()
            ));
        }
        Ok(lock)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string_pretty(self)
            .map_err(|error| format!("cannot serialize plugin lock: {error}"))?;
        write_atomically(path, &text)
    }

    pub fn set_enabled(&mut self, name: &str, enabled: bool) -> bool {
        let Some(plugin) = self.plugins.get_mut(name) else {
            return false;
        };
        plugin.enabled = enabled;
        true
    }

    pub fn remove(&mut self, name: &str) -> Option<LockedPlugin> {
        self.plugins.remove(name)
    }
}

pub fn load_managed_catalog(dir: &Path, lock: &PluginLock) -> PluginCatalog {
    let mut catalog = load_catalog(dir);
    let mut plugins = Vec::with_capacity(catalog.plugins.len());
    for plugin in catalog.plugins.drain(..) {
        let Some(locked) = lock.plugins.get(&plugin.manifest.name) else {
            catalog.warnings.push(warning(&plugin, MISSING_LOCK_ENTRY));
            continue;
        };
        if !locked.enabled {
            continue;
        }
        if plugin.component_hash() != Some(locked.sha256.as_str()) {
            catalog.warnings.push(warning(&plugin, HASH_MISMATCH));
            continue;
        }
        if plugin.manifest.protocol != locked.api {
            catalog.warnings.push(warning(&plugin, API_MISMATCH));
            continue;
        }
        plugins.push(plugin);
    }
    catalog.plugins = plugins;
    catalog
}

fn warning(plugin: &crate::manifest::LoadedPlugin, message: &str) -> PluginWarning {
    PluginWarning {
        path: plugin.path.clone(),
        message: message.to_string(),
    }
}

fn lock_version() -> u64 {
    LOCK_VERSION
}

fn default_enabled() -> bool {
    true
}

fn write_atomically(path: &Path, text: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    let temp = temp_path(path);
    fs::write(&temp, text).map_err(|error| format!("cannot write {}: {error}", temp.display()))?;
    fs::rename(&temp, path).map_err(|error| {
        let _ = fs::remove_file(&temp);
        format!("cannot replace {}: {error}", path.display())
    })
}

fn temp_path(path: &Path) -> PathBuf {
    let mut temp = path.as_os_str().to_os_string();
    temp.push(TEMP_SUFFIX);
    PathBuf::from(temp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_plugin_lock_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn locked() -> LockedPlugin {
        LockedPlugin {
            source: "owner/repo".to_string(),
            reference: Some("v1.0.0".to_string()),
            commit: "abc".to_string(),
            sha256: "sha256:def".to_string(),
            api: 1,
            enabled: true,
        }
    }

    #[test]
    fn lockfile_round_trips() {
        let dir = temp_dir("roundtrip");
        let path = dir.join("plugins.lock");
        let mut lock = PluginLock::default();
        lock.plugins.insert("echo".to_string(), locked());
        lock.save(&path).unwrap();
        assert_eq!(PluginLock::load(&path).unwrap().plugins["echo"], locked());
    }

    #[test]
    fn missing_lockfile_is_empty() {
        let dir = temp_dir("missing");
        assert!(PluginLock::load(&dir.join("plugins.lock"))
            .unwrap()
            .plugins
            .is_empty());
    }

    #[test]
    fn rejects_unknown_lockfile_versions() {
        let dir = temp_dir("version");
        let path = dir.join("plugins.lock");
        fs::write(&path, "version = 2\n").unwrap();
        assert!(PluginLock::load(&path)
            .unwrap_err()
            .contains("unsupported lockfile version"));
    }

    #[test]
    fn managed_catalog_loads_only_verified_entries() {
        let dir = temp_dir("managed");
        let hash = write_plugin(&dir);
        let mut lock = PluginLock::default();
        lock.plugins.insert(
            "echo".to_string(),
            LockedPlugin {
                sha256: hash,
                ..locked()
            },
        );
        let catalog = load_managed_catalog(&dir, &lock);
        assert_eq!(catalog.plugins.len(), 1);
        assert!(catalog.warnings.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn managed_catalog_ignores_unlisted_tampered_or_disabled_plugins() {
        let dir = temp_dir("managed-reject");
        let hash = write_plugin(&dir);
        let mut lock = PluginLock::default();
        assert!(load_managed_catalog(&dir, &lock).plugins.is_empty());

        lock.plugins.insert(
            "echo".to_string(),
            LockedPlugin {
                sha256: "sha256:0000".to_string(),
                ..locked()
            },
        );
        let catalog = load_managed_catalog(&dir, &lock);
        assert!(catalog.plugins.is_empty());
        assert!(catalog
            .warnings
            .iter()
            .any(|warning| warning.message.contains("hash")));

        lock.plugins.insert(
            "echo".to_string(),
            LockedPlugin {
                sha256: hash,
                enabled: false,
                ..locked()
            },
        );
        let catalog = load_managed_catalog(&dir, &lock);
        assert!(catalog.plugins.is_empty());
        assert!(catalog.warnings.is_empty());
        fs::remove_dir_all(dir).unwrap();
    }

    fn write_plugin(root: &Path) -> String {
        let component =
            fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/wasm/echo.wasm"))
                .unwrap();
        let hash = format!("sha256:{:x}", Sha256::digest(&component));
        let dir = root.join("echo");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("echo.wasm"), component).unwrap();
        fs::write(
            dir.join("plugin.toml"),
            format!(
                "name = \"echo\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"echo.wasm\"\nsha256 = \"{hash}\"\n"
            ),
        )
        .unwrap();
        hash
    }
}
