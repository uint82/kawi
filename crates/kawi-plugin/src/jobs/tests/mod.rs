mod events;
mod lifecycle;
mod limits;
mod output;
mod protocol;
mod validation;

use std::collections::BTreeMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::*;
use crate::capability::{Capability, EffectiveCapabilities, Scope};
use crate::manifest::{PluginManifest, PluginRuntime};
use crate::protocol::{
    JobExitedEvent, JobOutputEvent, StdinMode, JOB_EXITED_EVENT, JOB_OUTPUT_EVENT,
};

fn plugin(name: &str) -> LoadedPlugin {
    LoadedPlugin {
        manifest: PluginManifest {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            protocol: 1,
            description: String::new(),
            runtime: PluginRuntime::Process,
            component: None,
            sha256: None,
            command: Vec::new(),
            timeout_ms: 1_000,
            decorations: Vec::new(),
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

fn registry(name: &str, commands: &[&str]) -> CapabilityRegistry {
    let registry = CapabilityRegistry::default();
    registry.replace(BTreeMap::from([(
        name.to_string(),
        EffectiveCapabilities {
            granted: BTreeMap::from([(
                Capability::ProcessSpawn,
                Scope::Commands(commands.iter().map(|command| command.to_string()).collect()),
            )]),
        },
    )]));
    registry
}

fn manager(name: &str, commands: &[&str]) -> (JobManager, mpsc::Receiver<PluginHostEvent>) {
    let (events, receiver) = mpsc::channel();
    let (wake, _) = mpsc::channel();
    (
        JobManager::new(
            registry(name, commands),
            HostEnvironment::default(),
            events,
            wake,
        ),
        receiver,
    )
}

fn params(command: &str, args: &[&str]) -> ProcessOpenParams {
    ProcessOpenParams {
        command: command.to_string(),
        args: args.iter().map(|arg| arg.to_string()).collect(),
        cwd: None,
        env: BTreeMap::new(),
        stdin: StdinMode::Pipe,
    }
}

fn plugin_with_runtime(name: &str, runtime: PluginRuntime) -> LoadedPlugin {
    let mut plugin = plugin(name);
    plugin.manifest.runtime = runtime;
    plugin
}

fn wasm_plugin(name: &str) -> LoadedPlugin {
    plugin_with_runtime(name, PluginRuntime::Wasm)
}

fn open_with(manager: &JobManager, plugin: &LoadedPlugin, command: &str, args: &[&str]) -> u64 {
    manager
        .open(plugin, params(command, args))
        .expect("open job")
        .job
}

fn open(manager: &JobManager, name: &str, command: &str, args: &[&str]) -> u64 {
    open_with(manager, &plugin(name), command, args)
}

fn wait_exit(receiver: &mpsc::Receiver<PluginHostEvent>, job: u64) -> JobExitedEvent {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = receiver.recv_timeout(remaining).expect("job exit");
        if let PluginHostEvent::JobEvent { event, payload, .. } = event {
            if event == JOB_EXITED_EVENT {
                let exited: JobExitedEvent = serde_json::from_value(payload).expect("exit event");
                if exited.job == job {
                    return exited;
                }
            }
        }
    }
}

fn output_text(receiver: &mpsc::Receiver<PluginHostEvent>, job: u64) -> String {
    let mut text = String::new();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let event = receiver.recv_timeout(remaining).expect("job output");
        if let PluginHostEvent::JobEvent { event, payload, .. } = event {
            if event == JOB_OUTPUT_EVENT {
                let output: JobOutputEvent = serde_json::from_value(payload).expect("output event");
                if output.job == job {
                    text.push_str(&output.data);
                    if !text.is_empty() {
                        return text;
                    }
                }
            }
        }
    }
}
