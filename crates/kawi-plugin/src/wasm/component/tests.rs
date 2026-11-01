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
fn reuses_cached_components_by_hash() {
    let runtime = WasmRuntime::new().unwrap();
    let bytes = std::fs::read(fixture("echo.wasm")).unwrap();
    let (first, hash) = runtime.compile(&bytes, None).unwrap();
    let (second, second_hash) = runtime.compile(&bytes, None).unwrap();
    assert_eq!(hash, second_hash);
    assert!(std::sync::Arc::ptr_eq(&first, &second));
}

#[test]
fn evicts_the_oldest_cached_component() {
    let runtime = WasmRuntime::new().unwrap();
    let bytes = std::fs::read(fixture("echo.wasm")).unwrap();
    let (component, _) = runtime.compile(&bytes, None).unwrap();
    let mut cache = ComponentCache::new(1);
    cache.insert("first".to_string(), std::sync::Arc::clone(&component));
    cache.insert("second".to_string(), std::sync::Arc::clone(&component));
    assert!(cache.get("first").is_none());
    assert!(cache.get("second").is_some());
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
        vec!["kawi:plugin/editor-read@0.2.0".to_string()]
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
        vec!["kawi:plugin/editor-read@0.2.0".to_string()]
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
            &[
                Capability::EditorRead,
                Capability::FsRead,
                Capability::FsWrite,
            ],
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

#[test]
fn instantiates_and_calls_a_component() {
    use wasmtime::component::{Linker, ResourceTable};
    use wasmtime_wasi::{WasiCtx, WasiCtxView, WasiView};

    struct State {
        wasi: WasiCtx,
        table: ResourceTable,
    }

    impl WasiView for State {
        fn ctx(&mut self) -> WasiCtxView<'_> {
            WasiCtxView {
                ctx: &mut self.wasi,
                table: &mut self.table,
            }
        }
    }

    impl super::super::bindings::kawi::plugin::types::Host for State {}

    let runtime = WasmRuntime::new().unwrap();
    let path = fixture("echo.wasm");
    let bytes = std::fs::read(&path).unwrap();
    let (component, _) = runtime.compile(&bytes, None).unwrap();
    let mut linker = Linker::new(&runtime.engine);
    super::super::bindings::kawi::plugin::types::add_to_linker::<
        State,
        wasmtime::component::HasSelf<State>,
    >(&mut linker, |state| state)
    .unwrap();
    let mut store = wasmtime::Store::new(
        &runtime.engine,
        State {
            wasi: crate::wasm::WasiConfig::new().build(),
            table: ResourceTable::new(),
        },
    );
    store.set_epoch_deadline(100);
    let instance =
        super::super::bindings::Host_::instantiate(&mut store, &component, &linker).unwrap();
    let init = instance
        .kawi_plugin_lifecycle()
        .call_init(&mut store, 1)
        .unwrap();
    assert!(init.is_ok());
    let result = instance
        .kawi_plugin_lifecycle()
        .call_invoke_command(&mut store, "ping", &[])
        .unwrap()
        .unwrap();
    assert!(matches!(
        result,
        super::super::bindings::kawi::plugin::types::Argument::Text(value)
            if value == "pong"
    ));
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/wasm")
        .join(name)
}

#[test]
fn accepts_structural_type_imports() {
    let runtime = WasmRuntime::new().unwrap();
    let inspection = runtime.inspect(&fixture("echo.wasm"), None, &[]).unwrap();
    assert_eq!(
        inspection.imports,
        vec!["kawi:plugin/types@0.2.0".to_string()]
    );
}
