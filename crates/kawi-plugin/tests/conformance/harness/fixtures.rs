use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use sha2::{Digest, Sha256};

use super::super::scenario::Behavior;
use super::model::Runtime;
use super::PLUGIN_NAME;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn temp_root() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let root = std::env::temp_dir().join(format!("kawi-conformance-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

pub(crate) fn write_grants(path: &Path, grants: &[kawi_plugin::Capability]) {
    let mut text = String::new();
    if !grants.is_empty() {
        text.push_str(&format!("[plugins.{PLUGIN_NAME}.grant]\n"));
        for capability in grants {
            text.push_str(&format!("\"{}\" = true\n", capability.name()));
        }
    }
    fs::write(path, text).unwrap();
}

pub(crate) fn write_manifest(plugins: &Path, runtime: Runtime, behavior: Behavior) {
    let dir = plugins.join(PLUGIN_NAME);
    fs::create_dir_all(&dir).unwrap();
    let capabilities = capability_table(behavior);
    let subscribes = subscribes_line(behavior);
    let manifest = match runtime {
        Runtime::Process => process_manifest(behavior, &subscribes, &capabilities),
        Runtime::Wasm => wasm_manifest(&dir, behavior, &subscribes, &capabilities),
    };
    fs::write(dir.join("plugin.toml"), manifest).unwrap();
}

fn process_manifest(behavior: Behavior, subscribes: &str, capabilities: &str) -> String {
    let executable = env!("CARGO_BIN_EXE_fake-plugin");
    format!(
        "name = \"{PLUGIN_NAME}\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = [{executable:?}, {:?}]\ntimeout_ms = 2000\n{subscribes}\n{capabilities}\n[[commands]]\nid = \"probe\"\ntitle = \"Probe\"\n",
        behavior.process_mode()
    )
}

fn wasm_manifest(dir: &Path, behavior: Behavior, subscribes: &str, capabilities: &str) -> String {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/wasm")
        .join(behavior.wasm_component());
    let bytes = fs::read(source).unwrap();
    let bytes = if behavior.oversizes_component() {
        pad_component(bytes)
    } else {
        bytes
    };
    fs::write(dir.join("plugin.wasm"), &bytes).unwrap();
    let digest = Sha256::digest(&bytes);
    let hash = if behavior == Behavior::HashMismatch {
        "0".repeat(64)
    } else {
        format!("{digest:x}")
    };
    format!(
        "name = \"{PLUGIN_NAME}\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"plugin.wasm\"\nsha256 = \"sha256:{hash}\"\ntimeout_ms = 2000\n{subscribes}\n{capabilities}"
    )
}

fn subscribes_line(behavior: Behavior) -> String {
    match behavior.event() {
        Some(event) => format!("subscribes = [\"{event}\"]\n"),
        None => String::new(),
    }
}

fn pad_component(mut bytes: Vec<u8>) -> Vec<u8> {
    const MAX_COMPONENT_BYTES: usize = 64 * 1024 * 1024;
    const PADDING_NAME: &[u8] = b"conformance-padding";
    let payload_len = MAX_COMPONENT_BYTES
        .saturating_sub(bytes.len())
        .saturating_add(1);
    let mut section = Vec::new();
    write_leb128(PADDING_NAME.len(), &mut section);
    section.extend_from_slice(PADDING_NAME);
    section.resize(section.len() + payload_len, 0);
    bytes.push(0);
    write_leb128(section.len(), &mut bytes);
    bytes.extend_from_slice(&section);
    bytes
}

fn write_leb128(mut value: usize, output: &mut Vec<u8>) {
    loop {
        let mut byte = (value & 0x7f) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if value == 0 {
            break;
        }
    }
}

fn capability_table(behavior: Behavior) -> String {
    let requests = behavior.requests();
    if requests.is_empty() {
        return String::new();
    }
    let mut table = String::from("[capabilities]\n");
    for capability in requests {
        table.push_str(&format!("\"{}\" = true\n", capability.name()));
    }
    table
}
