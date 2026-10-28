use std::path::Path;

use sha2::{Digest, Sha256};
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
            .epoch_interruption(true)
            .max_wasm_stack(MAX_STACK_BYTES);
        let engine = Engine::new(&config)
            .map_err(|error| PluginError::Initialize(format!("wasm engine: {error}")))?;
        Ok(Self { engine })
    }

    pub fn inspect(
        &self,
        path: &Path,
        requested: &[Capability],
    ) -> Result<ComponentInspection, PluginError> {
        let bytes = std::fs::read(path)
            .map_err(|error| PluginError::Spawn(format!("cannot read component: {error}")))?;
        let component = Component::from_binary(&self.engine, &bytes)
            .map_err(|error| PluginError::Initialize(format!("invalid component: {error}")))?;
        let imports = component
            .component_type()
            .imports(&self.engine)
            .map(|(name, _)| name.to_string())
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
            hash: digest(&bytes),
        })
    }
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
            .inspect(&fixture("editor-read.wasm"), &[Capability::EditorRead])
            .unwrap();
        assert_eq!(
            inspection.imports,
            vec!["kawi:plugin/editor-read@0.1.0".to_string()]
        );
        assert_eq!(inspection.requested, vec![Capability::EditorRead]);
        assert!(inspection.hash.starts_with("sha256:"));
    }

    #[test]
    fn rejects_component_imports_without_a_request() {
        let runtime = WasmRuntime::new().unwrap();
        let error = runtime
            .inspect(&fixture("editor-and-fs.wasm"), &[Capability::EditorRead])
            .unwrap_err();
        assert!(error.to_string().contains("wasi:filesystem/types@0.2.12"));
    }

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/wasm")
            .join(name)
    }
}
