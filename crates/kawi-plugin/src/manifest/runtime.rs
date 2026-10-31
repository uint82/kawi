use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::validation::validate_command;
use super::PluginManifest;

const SHA256_PREFIX: &str = "sha256:";
const SHA256_HEX_LEN: usize = 64;
const COMPONENT_HEADER: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x0d, 0x00, 0x01, 0x00];
const PROCESS_LABEL: &str = "process (trusted compatibility runtime)";
const WASM_LABEL: &str = "wasm";

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PluginRuntime {
    #[default]
    Process,
    Wasm,
}

impl PluginRuntime {
    pub fn label(self) -> &'static str {
        match self {
            Self::Process => PROCESS_LABEL,
            Self::Wasm => WASM_LABEL,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct Artifact {
    pub computed_hash: Option<String>,
    pub disabled_reason: Option<String>,
}

impl Artifact {
    fn ready(computed_hash: Option<String>) -> Self {
        Self {
            computed_hash,
            disabled_reason: None,
        }
    }

    fn disabled(reason: String, computed_hash: Option<String>) -> Self {
        Self {
            computed_hash,
            disabled_reason: Some(reason),
        }
    }
}

pub(super) fn validate(manifest: &mut PluginManifest, dir: &Path) -> Result<Artifact, String> {
    match manifest.runtime {
        PluginRuntime::Process => {
            validate_process(manifest)?;
            Ok(Artifact::ready(None))
        }
        PluginRuntime::Wasm => Ok(validate_wasm(manifest, dir)),
    }
}

fn validate_process(manifest: &PluginManifest) -> Result<(), String> {
    if manifest.component.is_some() || manifest.sha256.is_some() {
        return Err("process plugins cannot declare component or sha256".to_string());
    }
    validate_command(&manifest.command)
}

fn validate_wasm(manifest: &mut PluginManifest, dir: &Path) -> Artifact {
    if !manifest.command.is_empty() {
        return Artifact::disabled("wasm plugins cannot declare command".to_string(), None);
    }
    let Some(component) = manifest.component.as_deref() else {
        return Artifact::disabled("wasm plugins require component".to_string(), None);
    };
    if let Err(message) = validate_component_path(component) {
        return Artifact::disabled(message, None);
    }
    let Some(declared) = manifest.sha256.as_deref() else {
        return Artifact::disabled("wasm plugins require sha256".to_string(), None);
    };
    let expected = match normalize_sha256(declared) {
        Ok(expected) => expected,
        Err(message) => return Artifact::disabled(message, None),
    };
    manifest.sha256 = Some(expected.clone());
    let path = match resolve_component_path(dir, component) {
        Ok(path) => path,
        Err(message) => return Artifact::disabled(message, None),
    };
    let (bytes, computed) = match read_component(&path) {
        Ok(component) => component,
        Err(message) => return Artifact::disabled(message, None),
    };
    if computed != expected {
        return Artifact::disabled(
            format!("component hash mismatch: expected {expected}, got {computed}"),
            Some(computed),
        );
    }
    if let Err(message) = validate_component_bytes(&bytes, &computed) {
        return Artifact::disabled(message, Some(computed));
    }
    Artifact::ready(Some(computed))
}

pub fn validate_component_path(component: &str) -> Result<(), String> {
    if component.trim().is_empty() {
        return Err("component must not be empty".to_string());
    }
    if component.contains('\\') {
        return Err("component must use forward slashes".to_string());
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

fn resolve_component_path(dir: &Path, component: &str) -> Result<PathBuf, String> {
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

fn read_component(path: &Path) -> Result<(Vec<u8>, String), String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read component {}: {error}", path.display()))?;
    let digest = Sha256::digest(&bytes);
    Ok((bytes, format!("sha256:{digest:x}")))
}

fn validate_component_bytes(bytes: &[u8], hash: &str) -> Result<(), String> {
    if !bytes.starts_with(&COMPONENT_HEADER) {
        return Err("not a WebAssembly component".to_string());
    }
    #[cfg(feature = "wasm")]
    {
        let runtime = crate::wasm::WasmRuntime::new()
            .map_err(|error| format!("invalid component: {error}"))?;
        runtime
            .validate_bytes(bytes, Some(hash))
            .map_err(|error| format!("invalid component: {error}"))
    }
    #[cfg(not(feature = "wasm"))]
    {
        let _ = (bytes, hash);
        Ok(())
    }
}
