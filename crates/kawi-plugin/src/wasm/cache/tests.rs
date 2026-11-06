use std::fs;
use std::path::{Path, PathBuf};

use wasmtime::component::Component;

use super::*;
use crate::wasm::WasmRuntime;

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kawi_wasm_cache_{}_{}", std::process::id(), name));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/wasm")
        .join(name)
}

fn component(runtime: &WasmRuntime) -> Component {
    let bytes = fs::read(fixture("echo.wasm")).unwrap();
    Component::from_binary(runtime.engine(), &bytes).unwrap()
}

#[test]
fn stored_components_round_trip_through_the_cache() {
    let dir = temp_dir("roundtrip");
    let runtime = WasmRuntime::new().unwrap();
    let component = component(&runtime);
    store(&dir, "sha256:roundtrip", &component).unwrap();
    assert!(component_path(&dir, "sha256:roundtrip").is_file());
    let loaded = load(runtime.engine(), &dir, "sha256:roundtrip").expect("cache hit");
    assert!(loaded
        .component_type()
        .imports(runtime.engine())
        .next()
        .is_some());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_components_are_cache_misses() {
    let dir = temp_dir("missing");
    let runtime = WasmRuntime::new().unwrap();
    assert!(load(runtime.engine(), &dir, "sha256:missing").is_none());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn corrupt_components_are_removed_after_a_miss() {
    let dir = temp_dir("corrupt");
    let runtime = WasmRuntime::new().unwrap();
    let path = component_path(&dir, "sha256:corrupt");
    fs::write(&path, b"not a component").unwrap();
    assert!(load(runtime.engine(), &dir, "sha256:corrupt").is_none());
    assert!(!path.exists());
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn store_failures_are_reported() {
    let dir = temp_dir("store_failure");
    let path = dir.join("not_a_directory");
    fs::write(&path, b"file").unwrap();
    let runtime = WasmRuntime::new().unwrap();
    let component = component(&runtime);
    assert!(store(&path, "sha256:failure", &component).is_err());
    fs::remove_dir_all(dir).unwrap();
}

#[cfg(unix)]
#[test]
fn cache_directories_and_files_are_private() {
    use std::os::unix::fs::PermissionsExt;

    let dir = temp_dir("permissions").join("cache");
    let runtime = WasmRuntime::new().unwrap();
    let component = component(&runtime);
    store(&dir, "sha256:permissions", &component).unwrap();
    assert_eq!(
        fs::metadata(&dir).unwrap().permissions().mode() & 0o777,
        0o700
    );
    let path = component_path(&dir, "sha256:permissions");
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}
