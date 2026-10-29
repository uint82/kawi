use std::path::Path;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

use sha2::{Digest, Sha256};
use wasmtime::component::types::{ComponentExtern, ComponentInstance, ComponentItem};
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

use crate::capability::Capability;
use crate::error::PluginError;

use super::policy::{requested_import_capabilities, unauthorized_imports};

const MAX_STACK_BYTES: usize = 512 * 1024;
const EPOCH_INTERVAL: Duration = Duration::from_millis(50);

static ENGINE: OnceLock<Result<Engine, String>> = OnceLock::new();

pub struct WasmRuntime {
    engine: Engine,
}

#[derive(Debug)]
pub struct ComponentInspection {
    pub imports: Vec<String>,
    pub requested: Vec<Capability>,
    pub hash: String,
}

impl WasmRuntime {
    pub fn new() -> Result<Self, PluginError> {
        let engine = ENGINE.get_or_init(build_engine);
        match engine {
            Ok(engine) => Ok(Self {
                engine: engine.clone(),
            }),
            Err(message) => Err(PluginError::Initialize(message.clone())),
        }
    }

    pub(crate) fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn validate_bytes(
        &self,
        bytes: &[u8],
        expected_hash: Option<&str>,
    ) -> Result<(), PluginError> {
        self.compile(bytes, expected_hash).map(|_| ())
    }

    fn compile(
        &self,
        bytes: &[u8],
        expected_hash: Option<&str>,
    ) -> Result<(Component, String), PluginError> {
        let hash = digest(bytes);
        if expected_hash.is_some_and(|expected| expected != hash) {
            return Err(PluginError::Initialize(format!(
                "component hash changed since load: expected {}, got {hash}",
                expected_hash.unwrap_or_default()
            )));
        }
        let component = Component::from_binary(&self.engine, bytes)
            .map_err(|error| PluginError::Initialize(format!("invalid component: {error}")))?;
        Ok((component, hash))
    }

    pub fn inspect(
        &self,
        path: &Path,
        expected_hash: Option<&str>,
        requested: &[Capability],
    ) -> Result<ComponentInspection, PluginError> {
        let bytes = std::fs::read(path)
            .map_err(|error| PluginError::Spawn(format!("cannot read component: {error}")))?;
        self.inspect_bytes(&bytes, expected_hash, requested)
    }

    pub(crate) fn inspect_bytes(
        &self,
        bytes: &[u8],
        expected_hash: Option<&str>,
        requested: &[Capability],
    ) -> Result<ComponentInspection, PluginError> {
        self.inspect_component(bytes, expected_hash, requested)
            .map(|(_, inspection)| inspection)
    }

    pub(crate) fn inspect_component(
        &self,
        bytes: &[u8],
        expected_hash: Option<&str>,
        requested: &[Capability],
    ) -> Result<(Component, ComponentInspection), PluginError> {
        let (component, hash) = self.compile(bytes, expected_hash)?;
        let inspection = self.check_imports(&component, hash, requested)?;
        Ok((component, inspection))
    }

    fn check_imports(
        &self,
        component: &Component,
        hash: String,
        requested: &[Capability],
    ) -> Result<ComponentInspection, PluginError> {
        let imports = component
            .component_type()
            .imports(&self.engine)
            .flat_map(|(name, item)| collect_import(name, &item, &self.engine))
            .collect::<Vec<_>>();
        let requested = requested
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let unauthorized = unauthorized_imports(imports.iter().map(String::as_str), &requested);
        if !unauthorized.is_empty() {
            return Err(PluginError::Initialize(format!(
                "component imports capabilities not requested: {}",
                unauthorized.join(", ")
            )));
        }
        let used = requested_import_capabilities(imports.iter().map(String::as_str))
            .into_iter()
            .collect();
        Ok(ComponentInspection {
            imports,
            requested: used,
            hash,
        })
    }
}

fn build_engine() -> Result<Engine, String> {
    let mut config = Config::new();
    config
        .wasm_component_model(true)
        .wasm_component_model_implements(true)
        .epoch_interruption(true)
        .max_wasm_stack(MAX_STACK_BYTES);
    let engine = Engine::new(&config).map_err(|error| format!("wasm engine: {error}"))?;
    spawn_epoch_ticker(engine.weak(), EPOCH_INTERVAL);
    Ok(engine)
}

fn spawn_epoch_ticker(engine: wasmtime::EngineWeak, interval: Duration) {
    let _ = thread::Builder::new()
        .name("kawi-wasm-epoch".to_string())
        .spawn(move || loop {
            thread::sleep(interval);
            let Some(engine) = engine.upgrade() else {
                break;
            };
            engine.increment_epoch();
        });
}

fn collect_import(name: &str, item: &ComponentExtern<'_>, engine: &Engine) -> Vec<String> {
    let effective = item.implements.unwrap_or(name);
    if is_interface_name(effective) {
        return vec![effective.to_string()];
    }
    let mut imports = Vec::new();
    match &item.ty {
        ComponentItem::ComponentInstance(instance) => {
            imports.extend(collect_instance_imports(effective, instance, engine));
        }
        ComponentItem::Component(component) => {
            imports.extend(component.imports(engine).flat_map(|(name, item)| {
                collect_import(&format!("{effective}/{name}"), &item, engine)
            }));
        }
        _ => imports.push(effective.to_string()),
    }
    imports
}

fn is_interface_name(name: &str) -> bool {
    name.contains(':') && name.contains('/')
}

fn collect_instance_imports(
    prefix: &str,
    instance: &ComponentInstance,
    engine: &Engine,
) -> Vec<String> {
    let exports = instance.exports(engine).collect::<Vec<_>>();
    if exports.is_empty() {
        return vec![prefix.to_string()];
    }
    exports
        .into_iter()
        .flat_map(|(name, item)| collect_import(&format!("{prefix}/{name}"), &item, engine))
        .collect()
}

fn digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("sha256:{digest:x}")
}

#[cfg(test)]
mod tests;
