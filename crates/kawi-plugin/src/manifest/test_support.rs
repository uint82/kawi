use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use sha2::{Digest, Sha256};

use super::MANIFEST_FILE;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

const ECHO_COMPONENT: &[u8] = include_bytes!("../../tests/fixtures/wasm/echo.wasm");

pub(super) fn temp_dir(prefix: &str) -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("kawi-plugin-{prefix}-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

pub(super) fn write_manifest(root: &Path, name: &str, body: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("plugin.wasm"), ECHO_COMPONENT).unwrap();
    fs::write(dir.join(MANIFEST_FILE), body).unwrap();
}

pub(super) fn valid_manifest(name: &str) -> String {
    let hash = format!("{:x}", Sha256::digest(ECHO_COMPONENT));
    format!(
        r#"name = "{name}"
version = "0.1.0"
protocol = 1
runtime = "wasm"
component = "plugin.wasm"
sha256 = "sha256:{hash}"

[[commands]]
id = "ping"
title = "Ping"

[commands.keybindings]
normal = ["<leader>pp"]
"#
    )
}
