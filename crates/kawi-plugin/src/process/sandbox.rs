use std::path::{Path, PathBuf};

use kawi_process::SandboxPolicy;

use crate::capability::{Capability, CapabilityRegistry, Scope};
use crate::environment::HostEnvironment;
use crate::manifest::LoadedPlugin;

#[cfg(target_os = "linux")]
const RUNTIME_READ_ONLY: &[&str] = &[
    "/usr",
    "/lib",
    "/lib64",
    "/bin",
    "/sbin",
    "/opt",
    "/nix/store",
    "/run/current-system/sw",
    "/etc/ld.so.cache",
    "/etc/ssl",
    "/etc/ca-certificates",
    "/etc/resolv.conf",
    "/etc/hosts",
    "/etc/nsswitch.conf",
    "/etc/localtime",
    "/dev/urandom",
];

#[cfg(target_os = "linux")]
const RUNTIME_READ_WRITE: &[&str] = &["/dev/null"];

pub(super) struct SandboxPlan {
    pub(super) policy: SandboxPolicy,
    pub(super) warnings: Vec<String>,
}

pub(super) fn policy(
    plugin: &LoadedPlugin,
    capabilities: &CapabilityRegistry,
    environment: &HostEnvironment,
) -> SandboxPlan {
    let mut planner = Planner::new(plugin, capabilities, environment);
    planner.add_runtime();
    planner.add_plugin_dir();
    planner.add_manifest_executable();
    planner.add_file_scope(Capability::FsRead);
    planner.add_file_scope(Capability::FsWrite);
    planner.add_command_scope();
    planner.finish()
}

struct Planner<'a> {
    plugin: &'a LoadedPlugin,
    capabilities: &'a CapabilityRegistry,
    environment: &'a HostEnvironment,
    policy: SandboxPolicy,
    warnings: Vec<String>,
}

impl<'a> Planner<'a> {
    fn new(
        plugin: &'a LoadedPlugin,
        capabilities: &'a CapabilityRegistry,
        environment: &'a HostEnvironment,
    ) -> Self {
        Self {
            plugin,
            capabilities,
            environment,
            policy: SandboxPolicy::default(),
            warnings: Vec::new(),
        }
    }

    fn add_runtime(&mut self) {
        #[cfg(target_os = "linux")]
        {
            self.policy
                .read_only
                .extend(existing_paths(RUNTIME_READ_ONLY));
            self.policy.read_only.push(PathBuf::from("/proc/self"));
            self.policy
                .read_write
                .extend(existing_paths(RUNTIME_READ_WRITE));
        }
    }

    fn add_plugin_dir(&mut self) {
        match existing(&self.plugin.dir) {
            Some(path) => self.policy.read_write.push(path),
            None => self.warn(&format!(
                "sandbox skipped plugin directory {}: path does not exist",
                self.plugin.dir.display()
            )),
        }
    }

    fn add_manifest_executable(&mut self) {
        let command = self.plugin.resolved_command();
        let Some(executable) = command.first() else {
            return;
        };
        let executable = executable.to_string_lossy();
        let Some(path) = resolve_command(&executable, &self.plugin.dir) else {
            self.warn(&format!(
                "sandbox skipped plugin executable {executable}: executable not found"
            ));
            return;
        };
        self.add_executable(&path);
        if path.file_name().is_some_and(|name| name == "env") {
            if let Some(inner) = command.get(1) {
                if let Some(inner) = resolve_command(&inner.to_string_lossy(), &self.plugin.dir) {
                    self.add_executable(&inner);
                }
            }
        }
    }

    fn add_file_scope(&mut self, capability: Capability) {
        let name = self.plugin.manifest.name.as_str();
        let Some(Scope::Paths(paths)) = self.capabilities.scope(name, capability) else {
            return;
        };
        for path in paths {
            self.add_file_path(capability, &path);
        }
    }

    fn add_file_path(&mut self, capability: Capability, raw: &str) {
        let Some(expanded) = expand_path(raw, self.plugin, self.environment) else {
            self.warn(&format!(
                "sandbox skipped granted {} path {raw}: variable is unset",
                capability.name()
            ));
            return;
        };
        let Some(path) = existing(&expanded) else {
            self.warn(&format!(
                "sandbox skipped granted {} path {raw}: path does not exist",
                capability.name()
            ));
            return;
        };
        if capability == Capability::FsWrite {
            self.policy.read_write.push(path);
        } else {
            self.policy.read_only.push(path);
        }
    }

    fn add_command_scope(&mut self) {
        let name = self.plugin.manifest.name.as_str();
        let Some(Scope::Commands(commands)) =
            self.capabilities.scope(name, Capability::ProcessSpawn)
        else {
            return;
        };
        for command in commands {
            let Some(path) = resolve_command(&command, &self.plugin.dir) else {
                self.warn(&format!(
                    "sandbox skipped granted process.spawn command {command}: executable not found"
                ));
                continue;
            };
            self.add_executable(&path);
        }
    }

    fn add_executable(&mut self, path: &Path) {
        if let Some(prefix) = runtime_prefix(path) {
            self.policy.read_only.push(prefix);
        }
        let Some(path) = existing(path) else {
            self.warn(&format!(
                "sandbox skipped executable {}: path does not exist",
                path.display()
            ));
            return;
        };
        if let Some(interpreter) = kawi_process::executable_interpreter(&path) {
            if let Some(interpreter) = existing(&interpreter) {
                if let Some(prefix) = runtime_prefix(&interpreter) {
                    self.policy.read_only.push(prefix);
                }
                self.policy.execute.push(interpreter);
            }
        }
        self.policy.execute.push(path);
    }

    fn warn(&mut self, line: &str) {
        self.warnings.push(line.to_string());
    }

    fn finish(self) -> SandboxPlan {
        SandboxPlan {
            policy: self.policy,
            warnings: self.warnings,
        }
    }
}

fn runtime_prefix(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let prefix = if parent
        .file_name()
        .is_some_and(|name| name == "bin" || name == "sbin")
    {
        parent.parent()?
    } else {
        parent
    };
    existing(prefix)
}

fn expand_path(raw: &str, plugin: &LoadedPlugin, environment: &HostEnvironment) -> Option<PathBuf> {
    let expanded = if let Some(rest) = raw.strip_prefix("$WORKSPACE") {
        environment
            .workspace
            .as_ref()?
            .join(rest.trim_start_matches('/'))
    } else if let Some(rest) = raw.strip_prefix("$PLUGIN_DIR") {
        plugin.dir.join(rest.trim_start_matches('/'))
    } else if let Some(rest) = raw.strip_prefix("$CONFIG") {
        environment
            .config
            .as_ref()?
            .join(rest.trim_start_matches('/'))
    } else if let Some(rest) = raw.strip_prefix("$HOME") {
        environment
            .home
            .as_ref()?
            .join(rest.trim_start_matches('/'))
    } else {
        PathBuf::from(raw)
    };
    Some(expanded)
}

fn resolve_command(command: &str, directory: &Path) -> Option<PathBuf> {
    if Path::new(command).is_absolute() {
        return Some(PathBuf::from(command));
    }
    if Path::new(command).components().count() > 1 {
        return Some(directory.join(command));
    }
    kawi_process::find_executable(command)
}

fn existing(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path)
        .ok()
        .filter(|path| path.exists())
}

#[cfg(target_os = "linux")]
fn existing_paths(paths: &[&str]) -> Vec<PathBuf> {
    paths
        .iter()
        .filter_map(|path| existing(Path::new(path)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_plugin_and_environment_paths() {
        let plugin = LoadedPlugin {
            manifest: crate::manifest::PluginManifest {
                name: "sample".to_string(),
                version: "0.1.0".to_string(),
                protocol: 1,
                description: String::new(),
                command: vec!["/bin/true".to_string()],
                timeout_ms: 1000,
                commands: Vec::new(),
                subscribes: Vec::new(),
                debounce: Default::default(),
                activation_events: Vec::new(),
                capabilities: Default::default(),
                capability_requests: Default::default(),
            },
            dir: PathBuf::from("/plugins/sample"),
            path: PathBuf::from("/plugins/sample/plugin.toml"),
        };
        let environment = HostEnvironment {
            workspace: Some(PathBuf::from("/work")),
            config: Some(PathBuf::from("/config")),
            home: Some(PathBuf::from("/home/user")),
        };
        assert_eq!(
            expand_path("$WORKSPACE/src", &plugin, &environment),
            Some(PathBuf::from("/work/src"))
        );
        assert_eq!(
            expand_path("$PLUGIN_DIR/data", &plugin, &environment),
            Some(PathBuf::from("/plugins/sample/data"))
        );
        assert_eq!(
            expand_path("$CONFIG/kawi", &plugin, &environment),
            Some(PathBuf::from("/config/kawi"))
        );
        assert_eq!(
            expand_path("$HOME/.cache", &plugin, &environment),
            Some(PathBuf::from("/home/user/.cache"))
        );
    }
}
