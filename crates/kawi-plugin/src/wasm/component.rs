use std::path::Path;

use sha2::{Digest, Sha256};
use wasmtime::component::types::{ComponentExtern, ComponentInstance, ComponentItem};
use wasmtime::component::Component;
use wasmtime::{Config, Engine};

use crate::capability::Capability;
use crate::error::PluginError;

use super::policy::{requested_import_capabilities, unauthorized_imports};

const MAX_STACK_BYTES: usize = 512 * 1024;

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
        let mut config = Config::new();
        config
            .wasm_component_model(true)
            .wasm_component_model_implements(true)
            .epoch_interruption(true)
            .max_wasm_stack(MAX_STACK_BYTES);
        let engine = Engine::new(&config)
            .map_err(|error| PluginError::Initialize(format!("wasm engine: {error}")))?;
        Ok(Self { engine })
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
        let (component, hash) = self.compile(&bytes, expected_hash)?;
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
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn hashes_component_bytes() {
        assert_eq!(
            digest(b"plugin"),
            "sha256:5e689e2b01672bf33996e75d5e372ff60c536ce1599a1458e867cd8f4bef5160"
        );
    }

    #[test]
    fn runtime_configures_the_engine() {
        assert!(WasmRuntime::new().is_ok());
    }

    #[test]
    fn inspects_component_imports_and_hash() {
        let runtime = WasmRuntime::new().unwrap();
        let inspection = runtime
            .inspect(
                &fixture("editor-read.wasm"),
                None,
                &[Capability::EditorRead],
            )
            .unwrap();
        assert_eq!(
            inspection.imports,
            vec!["kawi:plugin/editor-read@0.1.0".to_string()]
        );
        assert_eq!(inspection.requested, vec![Capability::EditorRead]);
        assert!(inspection.hash.starts_with("sha256:"));
    }

    #[test]
    fn inspects_renamed_imports_by_their_implements_name() {
        let runtime = WasmRuntime::new().unwrap();
        let inspection = runtime
            .inspect(
                &fixture("renamed-import.wasm"),
                None,
                &[Capability::EditorRead],
            )
            .unwrap();
        assert_eq!(
            inspection.imports,
            vec!["kawi:plugin/editor-read@0.1.0".to_string()]
        );
    }

    #[test]
    fn rejects_nested_imports_without_a_matching_capability() {
        let runtime = WasmRuntime::new().unwrap();
        let error = runtime
            .inspect(&fixture("nested-import.wasm"), None, &[])
            .unwrap_err();
        assert!(error.to_string().contains("alias/editor-read"));
    }

    #[test]
    fn accepts_components_with_no_imports() {
        let runtime = WasmRuntime::new().unwrap();
        let inspection = runtime.inspect(&fixture("empty.wasm"), None, &[]).unwrap();
        assert!(inspection.imports.is_empty());
        assert!(inspection.requested.is_empty());
    }

    #[test]
    fn rejects_a_component_that_changed_after_load() {
        let runtime = WasmRuntime::new().unwrap();
        let error = runtime
            .inspect(
                &fixture("editor-read.wasm"),
                Some("sha256:0000"),
                &[Capability::EditorRead],
            )
            .unwrap_err();
        assert!(error.to_string().contains("changed since load"));
    }

    #[test]
    fn rejects_malformed_components_without_panicking() {
        let runtime = WasmRuntime::new().unwrap();
        let path = std::env::temp_dir().join(format!("kawi-malformed-{}", std::process::id()));
        std::fs::write(&path, b"not a component").unwrap();
        let error = runtime.inspect(&path, None, &[]).unwrap_err();
        assert!(error.to_string().contains("invalid component"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn accepts_versioned_wasi_imports_with_a_grant() {
        let runtime = WasmRuntime::new().unwrap();
        let inspection = runtime
            .inspect(
                &fixture("editor-and-fs.wasm"),
                None,
                &[Capability::EditorRead, Capability::FsRead],
            )
            .unwrap();
        assert!(inspection
            .imports
            .contains(&"wasi:filesystem/types@0.2.12".to_string()));
    }

    #[test]
    fn rejects_component_imports_without_a_request() {
        let runtime = WasmRuntime::new().unwrap();
        let error = runtime
            .inspect(
                &fixture("editor-and-fs.wasm"),
                None,
                &[Capability::EditorRead],
            )
            .unwrap_err();
        assert!(error.to_string().contains("wasi:filesystem/types@0.2.12"));
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/wasm")
            .join(name)
    }
}
