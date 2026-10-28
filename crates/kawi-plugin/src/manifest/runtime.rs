use std::fs;
use std::path::Path;

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::validation::validate_command;
use super::PluginManifest;

const SHA256_PREFIX: &str = "sha256:";
const SHA256_HEX_LEN: usize = 64;

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginRuntime {
    #[default]
    Process,
    Wasm,
}

pub(super) fn validate(manifest: &mut PluginManifest, dir: &Path) -> Result<(), String> {
    match manifest.runtime {
        PluginRuntime::Process => validate_process(manifest),
        PluginRuntime::Wasm => validate_wasm(manifest, dir),
    }
}

fn validate_process(manifest: &PluginManifest) -> Result<(), String> {
    if manifest.component.is_some() || manifest.sha256.is_some() {
        return Err("process plugins cannot declare component or sha256".to_string());
    }
    validate_command(&manifest.command)
}

fn validate_wasm(manifest: &mut PluginManifest, dir: &Path) -> Result<(), String> {
    if !manifest.command.is_empty() {
        return Err("wasm plugins cannot declare command".to_string());
    }
    let component = manifest
        .component
        .as_deref()
        .ok_or_else(|| "wasm plugins require component".to_string())?;
    validate_component_path(component)?;
    let declared = manifest
        .sha256
        .as_deref()
        .ok_or_else(|| "wasm plugins require sha256".to_string())?;
    let expected = normalize_sha256(declared)?;
    let path = resolve_component_path(dir, component)?;
    let actual = hash_file(&path)?;
    if actual != expected {
        return Err(format!(
            "component hash mismatch: expected {expected}, got {actual}"
        ));
    }
    manifest.sha256 = Some(expected);
    Ok(())
}

fn validate_component_path(component: &str) -> Result<(), String> {
    if component.trim().is_empty() {
        return Err("component must not be empty".to_string());
    }
    let path = Path::new(component);
    if path.is_absolute() {
        return Err("component must be relative to the plugin directory".to_string());
    }
    if path.components().any(|part| {
        !matches!(
            part,
            std::path::Component::Normal(_) | std::path::Component::CurDir
        )
    }) {
        return Err("component must stay inside the plugin directory".to_string());
    }
    Ok(())
}

fn resolve_component_path(dir: &Path, component: &str) -> Result<std::path::PathBuf, String> {
    let root = fs::canonicalize(dir)
        .map_err(|error| format!("cannot resolve plugin directory: {error}"))?;
    let path = fs::canonicalize(dir.join(component))
        .map_err(|error| format!("cannot resolve component: {error}"))?;
    if !path.starts_with(&root) {
        return Err("component must stay inside the plugin directory".to_string());
    }
    Ok(path)
}

fn normalize_sha256(value: &str) -> Result<String, String> {
    let hex = value.strip_prefix(SHA256_PREFIX).unwrap_or(value);
    if hex.len() != SHA256_HEX_LEN || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("sha256 must be 64 hexadecimal characters".to_string());
    }
    Ok(format!("sha256:{}", hex.to_ascii_lowercase()))
}

fn hash_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read component {}: {error}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok(format!("sha256:{digest:x}"))
}
