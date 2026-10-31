use super::*;
use crate::capability::Capability;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn temp_dir() -> PathBuf {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir =
        std::env::temp_dir().join(format!("kawi-plugin-manifest-{}-{id}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write_manifest(root: &Path, name: &str, body: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join(MANIFEST_FILE), body).unwrap();
}

fn valid_manifest(name: &str) -> String {
    format!(
        r#"name = "{name}"
version = "0.1.0"
protocol = 1
command = ["python3", "${{plugin_dir}}/plugin.py"]

[[commands]]
id = "ping"
title = "Ping"

[commands.keybindings]
normal = ["<leader>pp"]
"#
    )
}

#[test]
fn loads_a_manifest_and_resolves_the_plugin_dir() {
    let root = temp_dir();
    write_manifest(&root, "echo", &valid_manifest("echo"));
    let catalog = load_catalog(&root);
    assert!(catalog.warnings.is_empty());
    let plugin = &catalog.plugins[0];
    assert_eq!(plugin.manifest.name, "echo");
    assert_eq!(plugin.manifest.timeout_ms, DEFAULT_TIMEOUT_MS);
    assert_eq!(
        plugin.manifest.commands[0].keybindings["normal"],
        ["<leader>pp"]
    );
    let command = plugin.resolved_command();
    assert_eq!(command[0], "python3");
    assert_eq!(
        command[1].to_string_lossy(),
        format!("{}/plugin.py", plugin.dir.display())
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn labels_process_and_wasm_runtimes() {
    assert_eq!(
        PluginRuntime::Process.label(),
        "process (trusted compatibility runtime)"
    );
    assert_eq!(PluginRuntime::Wasm.label(), "wasm");
}

#[test]
fn rejects_unknown_fields() {
    let root = temp_dir();
    let body = valid_manifest("echo").replacen("name = ", "unexpected = true\nname = ", 1);
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert_eq!(catalog.plugins.len(), 0);
    assert_eq!(catalog.warnings.len(), 1);
    assert!(
        catalog.warnings[0].message.contains("unknown field"),
        "{}",
        catalog.warnings[0].message
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_duplicate_plugin_names() {
    let root = temp_dir();
    write_manifest(&root, "first", &valid_manifest("echo"));
    write_manifest(&root, "second", &valid_manifest("echo"));
    let catalog = load_catalog(&root);
    assert_eq!(catalog.plugins.len(), 1);
    assert!(catalog.warnings[0]
        .message
        .contains("duplicate plugin name"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_timeout_and_protocol() {
    let root = temp_dir();
    let body = valid_manifest("echo").replace("protocol = 1", "protocol = 2");
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.warnings[0].message.contains("unsupported protocol"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_invalid_command_ids_and_modes() {
    let root = temp_dir();
    let body = valid_manifest("echo").replace("id = \"ping\"", "id = \"Ping\"");
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.warnings[0].message.contains("invalid character"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_directory_is_not_a_warning() {
    let root = temp_dir().join("missing");
    let catalog = load_catalog(&root);
    assert!(catalog.warnings.is_empty());
    assert!(catalog.plugins.is_empty());
}

#[test]
fn parses_subscriptions_and_activation_events() {
    let root = temp_dir();
    let body = valid_manifest("echo").replacen(
        "\n[[commands]]",
        r#"
subscribes = ["buffer_changed", "file_saved", "unknown_event"]

[debounce]
buffer_changed = 100

[[activation_events]]
event = "on_file_open"
filetypes = ["rust"]
globs = ["*.rs"]

[[activation_events]]
event = "on_command"

[[commands]]"#,
        1,
    );
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    let plugin = &catalog.plugins[0];
    assert_eq!(plugin.manifest.subscribes.len(), 3);
    assert_eq!(plugin.manifest.debounce["buffer_changed"], 100);
    assert_eq!(plugin.manifest.activation_events.len(), 2);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_debounce_for_unsubscribed_events() {
    let root = temp_dir();
    let body = valid_manifest("echo").replacen(
        "\n[[commands]]",
        "\n[debounce]\nbuffer_changed = 100\n\n[[commands]]",
        1,
    );
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.warnings[0].message.contains("unsubscribed event"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rejects_unknown_activation_events() {
    let root = temp_dir();
    let body = valid_manifest("echo").replacen(
        "\n[[commands]]",
        "\n[[activation_events]]\nevent = \"on_idle\"\n\n[[commands]]",
        1,
    );
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.warnings[0]
        .message
        .contains("unknown activation event"));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn parses_capability_requests() {
    let root = temp_dir();
    let body = valid_manifest("echo").replacen(
        "\n[[commands]]",
        "\n[capabilities]\n\"editor.read\" = true\n\"fs.read\" = { paths = [\"$WORKSPACE\"] }\n\n[[commands]]",
        1,
    );
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.warnings.is_empty(), "{:?}", catalog.warnings);
    let requests = &catalog.plugins[0].manifest.capability_requests;
    assert!(requests.contains(Capability::EditorRead));
    assert!(requests.contains(Capability::FsRead));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn warns_on_invalid_capability_requests_without_skipping_the_plugin() {
    let root = temp_dir();
    let body = valid_manifest("echo").replacen(
        "\n[[commands]]",
        "\n[capabilities]\n\"unknown.cap\" = true\n\"fs.read\" = true\n\n[[commands]]",
        1,
    );
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert_eq!(catalog.plugins.len(), 1);
    assert_eq!(catalog.warnings.len(), 2);
    assert!(catalog
        .warnings
        .iter()
        .all(|warning| warning.message.contains("capabilities:")));
    fs::remove_dir_all(root).unwrap();
}

mod wasm;

#[test]
fn rejects_command_gate_outside_plugin_requests() {
    let root = temp_dir();
    let body = valid_manifest("echo").replace(
        "[[commands]]\nid = \"ping\"\ntitle = \"Ping\"",
        "[[commands]]\nid = \"ping\"\ntitle = \"Ping\"\ncapabilities = [\"editor.write\"]",
    );
    write_manifest(&root, "echo", &body);
    let catalog = load_catalog(&root);
    assert!(catalog.plugins.is_empty());
    assert!(
        catalog.warnings[0]
            .message
            .contains("command ping requires editor.write"),
        "{}",
        catalog.warnings[0].message
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn includes_command_gates_in_the_request_hash() {
    let ungated_root = temp_dir();
    write_manifest(&ungated_root, "echo", &valid_manifest("echo"));
    let gated_root = temp_dir();
    let body = valid_manifest("echo")
        .replace(
            "[[commands]]",
            "[capabilities]\n\"editor.write\" = true\n\n[[commands]]",
        )
        .replace(
            "[[commands]]\nid = \"ping\"\ntitle = \"Ping\"",
            "[[commands]]\nid = \"ping\"\ntitle = \"Ping\"\ncapabilities = [\"editor.write\"]",
        );
    write_manifest(&gated_root, "echo", &body);
    let ungated = load_catalog(&ungated_root);
    let gated = load_catalog(&gated_root);
    assert_ne!(
        ungated.plugins[0]
            .manifest
            .capability_requests
            .request_hash(),
        gated.plugins[0].manifest.capability_requests.request_hash()
    );
    assert_eq!(
        gated.plugins[0]
            .manifest
            .capability_requests
            .command_gate("ping")
            .map(|gate| gate.iter().copied().collect::<Vec<_>>()),
        Some(vec![Capability::EditorWrite])
    );
    fs::remove_dir_all(gated_root).unwrap();
    fs::remove_dir_all(ungated_root).unwrap();
}
