use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use kawi_plugin::{
    load_catalog, validate_component_path, Capability, CapabilityRequests, LockedPlugin, Scope,
};

use super::github::{ReleaseClient, ResolvedRelease};
use super::source::PluginSpec;

const MANIFEST_FILE: &str = "plugin.toml";
const STAGING_PREFIX: &str = ".kawi-plugin-stage-";
static NEXT_STAGE: AtomicU64 = AtomicU64::new(1);

pub(crate) struct PreparedPlugin {
    pub(crate) name: String,
    pub(crate) version: String,
    pub(crate) spec: PluginSpec,
    pub(crate) tag: String,
    pub(crate) commit: String,
    pub(crate) author: String,
    pub(crate) sha256: String,
    pub(crate) api: u64,
    pub(crate) requests: CapabilityRequests,
    pub(crate) lock: LockedPlugin,
    pub(crate) destination: PathBuf,
    _temp: TempDir,
    plugin_dir: PathBuf,
}

pub(crate) struct PreparedUpdate {
    pub(crate) plugin: PreparedPlugin,
    pub(crate) added: Vec<Capability>,
    pub(crate) widened: Vec<Capability>,
    pub(crate) requires_confirmation: bool,
    pub(crate) current: bool,
}

impl PreparedUpdate {
    pub(crate) fn changes(&self) -> Vec<Capability> {
        let mut changes = self.added.clone();
        for capability in &self.widened {
            if !changes.contains(capability) {
                changes.push(*capability);
            }
        }
        changes.sort();
        changes
    }
}

pub(crate) fn prepare(
    spec: &PluginSpec,
    client: &dyn ReleaseClient,
    installed_dir: &Path,
) -> Result<PreparedPlugin, String> {
    let release = client.resolve(spec)?;
    let temp = TempDir::create(installed_dir)?;
    let plugin_dir = temp.path().join("plugin");
    fs::create_dir(&plugin_dir)
        .map_err(|error| format!("cannot create {}: {error}", plugin_dir.display()))?;
    write_manifest(&plugin_dir, &release)?;
    write_component(&plugin_dir, &release, client)?;
    let catalog = load_catalog(temp.path());
    if let Some(warning) = catalog.warnings.first() {
        return Err(warning.to_string());
    }
    let loaded = catalog
        .plugins
        .first()
        .ok_or_else(|| "downloaded plugin has no manifest".to_string())?;
    if let Some(reason) = &loaded.disabled_reason {
        return Err(reason.clone());
    }
    let sha256 = loaded
        .component_hash()
        .ok_or_else(|| "wasm plugin has no component hash".to_string())?
        .to_string();
    let manifest = &loaded.manifest;
    let lock = LockedPlugin {
        source: spec.source(),
        reference: spec.reference.clone(),
        commit: release.commit.clone(),
        sha256: sha256.clone(),
        api: manifest.protocol,
        enabled: true,
    };
    Ok(PreparedPlugin {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        spec: spec.clone(),
        tag: release.tag.clone(),
        commit: release.commit.clone(),
        author: release.author.clone(),
        sha256,
        api: manifest.protocol,
        requests: manifest.capability_requests.clone(),
        lock,
        destination: installed_dir.join(&manifest.name),
        _temp: temp,
        plugin_dir,
    })
}

pub(crate) fn update(prepared: PreparedPlugin, previous: &CapabilityRequests) -> PreparedUpdate {
    let added = prepared.requests.added_capabilities(previous);
    let widened = prepared
        .requests
        .widened_capabilities(previous)
        .into_iter()
        .filter(|capability| !added.contains(capability))
        .collect::<Vec<_>>();
    let requires_confirmation = !added.is_empty() || !widened.is_empty();
    PreparedUpdate {
        plugin: prepared,
        added,
        widened,
        requires_confirmation,
        current: false,
    }
}

pub(crate) fn apply(prepared: &PreparedPlugin) -> Result<InstalledSwap, String> {
    let destination = prepared.destination.clone();
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let backup = backup_path(&destination);
    let had_backup = destination.exists();
    if had_backup {
        fs::rename(&destination, &backup)
            .map_err(|error| format!("cannot stage {}: {error}", destination.display()))?;
    }
    if let Err(error) = fs::rename(&prepared.plugin_dir, &destination) {
        if had_backup {
            let _ = fs::rename(&backup, &destination);
        }
        return Err(format!(
            "cannot install plugin into {}: {error}",
            destination.display()
        ));
    }
    Ok(InstalledSwap {
        destination,
        backup: had_backup.then_some(backup),
    })
}

pub(crate) struct InstalledSwap {
    destination: PathBuf,
    backup: Option<PathBuf>,
}

impl InstalledSwap {
    pub(crate) fn commit(mut self) -> Result<(), String> {
        let Some(backup) = self.backup.take() else {
            return Ok(());
        };
        fs::remove_dir_all(&backup)
            .map_err(|error| format!("cannot remove {}: {error}", backup.display()))
    }

    pub(crate) fn rollback(mut self) {
        let _ = fs::remove_dir_all(&self.destination);
        if let Some(backup) = self.backup.take() {
            let _ = fs::rename(backup, &self.destination);
        }
    }
}

pub(crate) fn remove_installed(name: &str, installed_dir: &Path) -> Result<(), String> {
    let path = installed_dir.join(name);
    match fs::remove_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("cannot remove {}: {error}", path.display())),
    }
}

pub(crate) fn capability_lines(requests: &CapabilityRequests) -> Vec<String> {
    requests
        .iter()
        .map(|(capability, scope)| format!("{} {}", capability.name(), scope_label(scope)))
        .collect()
}

pub(crate) fn manifest_requests(installed_dir: &Path, name: &str) -> Option<CapabilityRequests> {
    load_catalog(installed_dir)
        .plugins
        .into_iter()
        .find(|plugin| plugin.manifest.name == name)
        .map(|plugin| plugin.manifest.capability_requests)
}

fn write_manifest(plugin_dir: &Path, release: &ResolvedRelease) -> Result<(), String> {
    fs::write(plugin_dir.join(MANIFEST_FILE), &release.manifest)
        .map_err(|error| format!("cannot write manifest: {error}"))
}

fn write_component(
    plugin_dir: &Path,
    release: &ResolvedRelease,
    client: &dyn ReleaseClient,
) -> Result<(), String> {
    validate_component_path(&release.component)?;
    let path = plugin_dir.join(&release.component);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let bytes = client.download(release)?;
    fs::write(&path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

fn scope_label(scope: &Scope) -> String {
    match scope {
        Scope::Unscoped => "unscoped".to_string(),
        Scope::Paths(paths) => format!("paths {paths:?}"),
        Scope::Commands(commands) => format!("commands {commands:?}"),
        Scope::EditorCommands(commands) => format!("editor commands {commands:?}"),
        Scope::Hosts(hosts) => format!("hosts {hosts:?}"),
    }
}

fn backup_path(destination: &Path) -> PathBuf {
    let mut path = destination.as_os_str().to_os_string();
    path.push(".kawi-backup");
    PathBuf::from(path)
}

struct TempDir {
    path: PathBuf,
}

impl TempDir {
    fn create(parent: &Path) -> Result<Self, String> {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        let id = NEXT_STAGE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!("{STAGING_PREFIX}{}-{id}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir(&path)
            .map_err(|error| format!("cannot create {}: {error}", path.display()))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
