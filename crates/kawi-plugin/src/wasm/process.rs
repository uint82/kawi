use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use crate::capability::{Capability, CapabilityRegistry, Scope};
use crate::environment::HostEnvironment;
use crate::manifest::LoadedPlugin;
use crate::protocol::{RpcError, PLUGIN_UNAVAILABLE};

use super::bindings::kawi::plugin::types::ProcessOutput;
use super::convert::process_output;

const MIN_TIMEOUT_MS: u32 = 100;
const MAX_TIMEOUT_MS: u32 = 300_000;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

pub struct ProcessRunner {
    plugin: LoadedPlugin,
    capabilities: CapabilityRegistry,
    environment: HostEnvironment,
}

impl ProcessRunner {
    pub fn new(
        plugin: LoadedPlugin,
        capabilities: CapabilityRegistry,
        environment: HostEnvironment,
    ) -> Self {
        Self {
            plugin,
            capabilities,
            environment,
        }
    }

    pub fn run(
        &self,
        command: &str,
        args: &[String],
        timeout_ms: u32,
    ) -> Result<ProcessOutput, RpcError> {
        self.authorize(command)?;
        let path =
            kawi_process::find_executable_in(command, &self.plugin.dir).ok_or_else(|| {
                RpcError::new(PLUGIN_UNAVAILABLE, format!("command not found: {command}"))
            })?;
        let timeout =
            Duration::from_millis(u64::from(timeout_ms.clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS)));
        let mut command = Command::new(path);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        kawi_process::configure(&mut command);
        let policy = self.sandbox();
        kawi_process::configure_sandbox(&mut command, &policy);
        let mut child = command
            .spawn()
            .map_err(|error| RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()))?;
        let stdout = read_stream(child.stdout.take());
        let stderr = read_stream(child.stderr.take());
        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) if Instant::now() < deadline => thread::sleep(POLL_INTERVAL),
                Ok(None) => {
                    kawi_process::terminate(&mut child);
                    return Err(RpcError::new(PLUGIN_UNAVAILABLE, "process timed out"));
                }
                Err(error) => {
                    kawi_process::terminate(&mut child);
                    return Err(RpcError::new(PLUGIN_UNAVAILABLE, error.to_string()));
                }
            }
        };
        let stdout = join_stream(stdout);
        let stderr = join_stream(stderr);
        Ok(process_output(stdout, stderr, status.code()))
    }

    fn authorize(&self, command: &str) -> Result<(), RpcError> {
        let name = &self.plugin.manifest.name;
        if !self.capabilities.has(name, Capability::ProcessSpawn) {
            return Err(RpcError::capability_denied(
                Capability::ProcessSpawn.name(),
                None,
            ));
        }
        match self.capabilities.scope(name, Capability::ProcessSpawn) {
            Some(Scope::Commands(commands)) if commands.iter().any(|item| item == command) => {
                Ok(())
            }
            _ => Err(RpcError::capability_denied(
                Capability::ProcessSpawn.name(),
                Some(serde_json::json!({ "command": command })),
            )),
        }
    }

    pub fn sandbox(&self) -> kawi_process::SandboxPolicy {
        crate::process::sandbox_policy(&self.plugin, &self.capabilities, &self.environment).policy
    }
}

fn read_stream(stream: Option<impl Read + Send + 'static>) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut stream) = stream {
            let _ = stream.read_to_end(&mut bytes);
        }
        String::from_utf8_lossy(&bytes).into_owned()
    })
}

fn join_stream(stream: thread::JoinHandle<String>) -> String {
    stream.join().unwrap_or_default()
}

#[cfg(test)]
mod tests {
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
            EffectiveCapabilities {
                granted,
                ..Default::default()
            },
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
}
