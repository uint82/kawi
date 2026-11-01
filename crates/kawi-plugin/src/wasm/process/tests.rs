use std::collections::BTreeMap;

use super::*;
use crate::capability::{EffectiveCapabilities, Scope};
use crate::manifest::{PluginManifest, PluginRuntime};

fn plugin() -> LoadedPlugin {
    LoadedPlugin {
        manifest: PluginManifest {
            name: "runner".to_string(),
            version: "0.1.0".to_string(),
            protocol: 1,
            description: String::new(),
            runtime: PluginRuntime::Wasm,
            component: None,
            sha256: None,
            command: Vec::new(),
            timeout_ms: 1_000,
            settings: Default::default(),
            commands: Vec::new(),
            subscribes: Vec::new(),
            debounce: Default::default(),
            activation_events: Vec::new(),
            capabilities: toml::Table::new(),
            capability_requests: Default::default(),
        },
        dir: std::env::temp_dir(),
        path: std::env::temp_dir().join("plugin.toml"),
        computed_hash: None,
        disabled_reason: None,
    }
}

fn registry(granted: Option<Scope>) -> CapabilityRegistry {
    let registry = CapabilityRegistry::default();
    let granted = granted
        .map(|scope| BTreeMap::from([(Capability::ProcessSpawn, scope)]))
        .unwrap_or_default();
    registry.replace(BTreeMap::from([(
        "runner".to_string(),
        EffectiveCapabilities { granted },
    )]));
    registry
}

#[test]
fn runs_an_authorized_command() {
    let runner = ProcessRunner::new(
        plugin(),
        registry(Some(Scope::Commands(vec!["/bin/sh".to_string()]))),
        HostEnvironment::default(),
    );
    let output = runner
        .run(
            "/bin/sh",
            &["-c".to_string(), "printf hi".to_string()],
            1_000,
        )
        .unwrap();
    assert!(output.success);
    assert_eq!(output.stdout, "hi");
}

#[test]
fn rejects_a_command_outside_the_scope() {
    let runner = ProcessRunner::new(
        plugin(),
        registry(Some(Scope::Commands(vec!["/bin/true".to_string()]))),
        HostEnvironment::default(),
    );
    let error = runner
        .run("/bin/sh", &["-c".to_string(), "true".to_string()], 1_000)
        .unwrap_err();
    assert_eq!(error.code, crate::protocol::CAPABILITY_DENIED);
}

#[test]
fn rejects_an_ungranted_command_with_the_capability_error() {
    let runner = ProcessRunner::new(
        plugin(),
        registry(Some(Scope::Commands(vec!["/bin/true".to_string()]))),
        HostEnvironment::default(),
    );
    let error = runner.run("/bin/false", &[], 1_000).unwrap_err();
    assert_eq!(error.code, -32012);
}

#[test]
fn passes_arguments_without_a_shell() {
    let marker = std::env::temp_dir().join(format!("kawi-process-shell-{}", std::process::id()));
    let _ = std::fs::remove_file(&marker);
    let runner = ProcessRunner::new(
        plugin(),
        registry(Some(Scope::Commands(vec!["/bin/echo".to_string()]))),
        HostEnvironment::default(),
    );
    let argument = format!(";touch {}", marker.display());
    let output = runner
        .run("/bin/echo", std::slice::from_ref(&argument), 1_000)
        .unwrap();
    assert_eq!(output.stdout.trim(), argument);
    assert!(!marker.exists());
}

#[test]
fn caps_process_output() {
    let runner = ProcessRunner::new(
        plugin(),
        registry(Some(Scope::Commands(vec![
            "/bin/sh".to_string(),
            "yes".to_string(),
            "head".to_string(),
        ]))),
        HostEnvironment::default(),
    );
    let output = runner
        .run(
            "/bin/sh",
            &["-c".to_string(), "yes x | head -c 100000".to_string()],
            2_000,
        )
        .unwrap();
    assert!(output.success);
    assert_eq!(output.stdout.len(), MAX_OUTPUT_BYTES);
}

#[test]
fn terminates_a_process_that_exceeds_its_timeout() {
    let runner = ProcessRunner::new(
        plugin(),
        registry(Some(Scope::Commands(vec!["/bin/sh".to_string()]))),
        HostEnvironment::default(),
    );
    let started = Instant::now();
    let error = runner
        .run(
            "/bin/sh",
            &["-c".to_string(), "while :; do :; done".to_string()],
            100,
        )
        .unwrap_err();
    assert!(error.message.contains("timed out"));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn resolves_a_bare_grant_at_call_time() {
    let runner = ProcessRunner::new(
        plugin(),
        registry(Some(Scope::Commands(vec!["echo".to_string()]))),
        HostEnvironment::default(),
    );
    let output = runner.run("/bin/echo", &["ok".to_string()], 1_000).unwrap();
    assert_eq!(output.stdout.trim(), "ok");
}

#[test]
fn rejects_relative_commands_at_call_time() {
    let runner = ProcessRunner::new(
        plugin(),
        registry(Some(Scope::Commands(vec!["/bin/sh".to_string()]))),
        HostEnvironment::default(),
    );
    let error = runner.run("./sh", &[], 1_000).unwrap_err();
    assert_eq!(error.code, crate::protocol::PLUGIN_UNAVAILABLE);
}
