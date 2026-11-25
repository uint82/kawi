use std::path::{Path, PathBuf};

use kawi_process::SandboxPolicy;

use crate::capability::{Capability, CapabilityRegistry, Scope};
use crate::environment::{expand_path, HostEnvironment};
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

pub(crate) struct SandboxPlan {
    pub(crate) policy: SandboxPolicy,
}

pub(crate) fn policy(
    plugin: &LoadedPlugin,
    capabilities: &CapabilityRegistry,
    environment: &HostEnvironment,
) -> SandboxPlan {
    let mut planner = Planner::new(plugin, capabilities, environment);
    planner.add_runtime();
    planner.add_plugin_dir();
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
        if let Some(path) = existing(&self.plugin.dir) {
            self.policy.read_write.push(path);
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
        let Some(expanded) = expand_path(raw, &self.plugin.dir, self.environment) else {
            return;
        };
        let Some(path) = existing(&expanded) else {
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

    fn finish(self) -> SandboxPlan {
        SandboxPlan {
            policy: self.policy,
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
