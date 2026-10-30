use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use sha2::{Digest, Sha256};

use super::super::scenario::{network_case, Behavior, NetworkGrant, Scenario, PUBLIC_GRANT};
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

pub(crate) struct NetworkSetup {
    pub(crate) workspace: PathBuf,
    pub(crate) request: String,
    pub(crate) grant: Option<String>,
}

impl NetworkSetup {
    pub(crate) fn new(
        workspace: PathBuf,
        scenario: &Scenario,
        allowed: SocketAddr,
        hostname: String,
    ) -> Self {
        let case = network_case(scenario.id);
        let request = match case.grant {
            NetworkGrant::Local => allowed.to_string(),
            NetworkGrant::Hostname => hostname.clone(),
            NetworkGrant::Public | NetworkGrant::None => PUBLIC_GRANT.to_string(),
        };
        let grant = match case.grant {
            NetworkGrant::Local => Some(allowed.to_string()),
            NetworkGrant::Hostname => Some(hostname),
            NetworkGrant::Public => Some(PUBLIC_GRANT.to_string()),
            NetworkGrant::None => None,
        };
        Self {
            workspace,
            request,
            grant,
        }
    }
}

pub(crate) struct ManifestSpec<'a> {
    pub(crate) runtime: Runtime,
    pub(crate) scenario: &'a Scenario,
    pub(crate) network: Option<&'a NetworkSetup>,
}

pub(crate) fn write_grants(
    path: &Path,
    grants: &[kawi_plugin::Capability],
    network: Option<&NetworkSetup>,
    catalog: &kawi_plugin::PluginCatalog,
) {
    let mut text = approved_grants(catalog);
    text.push_str(&match network {
        Some(network) => network_grants(network),
        None => unscoped_grants(grants),
    });
    fs::write(path, text).unwrap();
}

fn approved_grants(catalog: &kawi_plugin::PluginCatalog) -> String {
    let mut text = String::new();
    for plugin in &catalog.plugins {
        let requests = &plugin.manifest.capability_requests;
        if requests.is_empty() {
            continue;
        }
        text.push_str(&format!("[plugins.{}.approved]\n", plugin.manifest.name));
        for (capability, scope) in requests.iter() {
            text.push_str(&approved_line(capability, scope));
        }
        text.push('\n');
    }
    text
}

fn approved_line(capability: kawi_plugin::Capability, scope: &kawi_plugin::Scope) -> String {
    match scope {
        kawi_plugin::Scope::Unscoped => format!("\"{}\" = true\n", capability.name()),
        kawi_plugin::Scope::Paths(paths) => {
            format!("\"{}\" = {{ paths = {paths:?} }}\n", capability.name())
        }
        kawi_plugin::Scope::Commands(commands) => {
            format!(
                "\"{}\" = {{ commands = {commands:?} }}\n",
                capability.name()
            )
        }
        kawi_plugin::Scope::Hosts(hosts) => {
            format!("\"{}\" = {{ hosts = {hosts:?} }}\n", capability.name())
        }
    }
}

fn unscoped_grants(grants: &[kawi_plugin::Capability]) -> String {
    let mut text = String::new();
    if !grants.is_empty() {
        text.push_str(&format!("[plugins.{PLUGIN_NAME}.grant]\n"));
        for capability in grants {
            text.push_str(&format!("\"{}\" = true\n", capability.name()));
        }
    }
    text
}

fn network_grants(network: &NetworkSetup) -> String {
    let Some(grant) = &network.grant else {
        return String::new();
    };
    let workspace = network.workspace.display();
    format!(
        "[plugins.{PLUGIN_NAME}.grant]\n\"fs.read\" = {{ paths = [{workspace:?}] }}\n\"net.connect\" = {{ hosts = [{grant:?}] }}\n"
    )
}

pub(crate) fn write_manifest(plugins: &Path, spec: ManifestSpec<'_>) {
    let behavior = spec.scenario.behavior;
    let timeout_ms = spec.scenario.timeout_ms();
    let dir = plugins.join(PLUGIN_NAME);
    fs::create_dir_all(&dir).unwrap();
    let capabilities = capability_table(behavior, spec.network);
    let subscribes = subscribes_line(behavior);
    let manifest = match spec.runtime {
        Runtime::Process => {
            process_manifest(&dir, behavior, timeout_ms, &subscribes, &capabilities)
        }
        Runtime::Wasm => wasm_manifest(&dir, behavior, timeout_ms, &subscribes, &capabilities),
    };
    fs::write(dir.join("plugin.toml"), manifest).unwrap();
}

fn process_command(directory: &Path, behavior: Behavior) -> String {
    if behavior == Behavior::Trim {
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins/trim/plugin.py");
        let script = directory.join("trim.py");
        fs::copy(source, &script).unwrap();
        return format!("[\"python3\", {:?}]", script.to_string_lossy());
    }
    let executable = env!("CARGO_BIN_EXE_fake-plugin");
    format!("[{executable:?}, {:?}]", behavior.process_mode())
}

fn process_manifest(
    directory: &Path,
    behavior: Behavior,
    timeout_ms: u64,
    subscribes: &str,
    capabilities: &str,
) -> String {
    let command = process_command(directory, behavior);
    format!(
        "name = \"{PLUGIN_NAME}\"\nversion = \"0.1.0\"\nprotocol = 1\ncommand = {command}\ntimeout_ms = {timeout_ms}\n{subscribes}\n{capabilities}\n[[commands]]\nid = \"probe\"\ntitle = \"Probe\"\n"
    )
}

fn wasm_manifest(
    dir: &Path,
    behavior: Behavior,
    timeout_ms: u64,
    subscribes: &str,
    capabilities: &str,
) -> String {
    let source = wasm_component_source(behavior);
    let bytes = fs::read(&source)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", source.display()));
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
        "name = \"{PLUGIN_NAME}\"\nversion = \"0.1.0\"\nprotocol = 1\nruntime = \"wasm\"\ncomponent = \"plugin.wasm\"\nsha256 = \"sha256:{hash}\"\ntimeout_ms = {timeout_ms}\n{subscribes}\n{capabilities}"
    )
}

fn wasm_component_source(behavior: Behavior) -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    if behavior == Behavior::Trim {
        return root.join("../../examples/wasm-trim/trim.wasm");
    }
    root.join("tests/fixtures/wasm")
        .join(behavior.wasm_component())
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

fn capability_table(behavior: Behavior, network: Option<&NetworkSetup>) -> String {
    if let Some(network) = network {
        return network_capability_table(network);
    }
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

fn network_capability_table(network: &NetworkSetup) -> String {
    let workspace = network.workspace.display();
    let request = &network.request;
    format!(
        "[capabilities]\n\"fs.read\" = {{ paths = [{workspace:?}] }}\n\"fs.write\" = {{ paths = [{workspace:?}] }}\n\"net.connect\" = {{ hosts = [{request:?}] }}\n"
    )
}
