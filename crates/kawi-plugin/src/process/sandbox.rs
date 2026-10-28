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

pub(super) fn policy(
    plugin: &LoadedPlugin,
    capabilities: &CapabilityRegistry,
    environment: &HostEnvironment,
) -> SandboxPolicy {
    let name = plugin.manifest.name.as_str();
    let mut policy = SandboxPolicy::default();
    add_runtime(&mut policy);
    add_plugin_dir(&mut policy, plugin);
    add_runtime_executable(&mut policy, plugin);
    add_file_scope(
        &mut policy,
        capabilities,
        name,
        Capability::FsRead,
        environment,
        plugin,
    );
    add_file_scope(
        &mut policy,
        capabilities,
        name,
        Capability::FsWrite,
        environment,
        plugin,
    );
    add_command_scope(&mut policy, capabilities, plugin);
    policy
}

fn add_runtime(policy: &mut SandboxPolicy) {
    #[cfg(target_os = "linux")]
    {
        policy.read_only.extend(existing_paths(RUNTIME_READ_ONLY));
        policy.read_only.push(PathBuf::from("/proc/self"));
        policy.read_write.extend(existing_paths(RUNTIME_READ_WRITE));
    }
}

fn add_plugin_dir(policy: &mut SandboxPolicy, plugin: &LoadedPlugin) {
    if let Some(path) = existing(&plugin.dir) {
        policy.read_write.push(path);
    }
}

fn add_runtime_executable(policy: &mut SandboxPolicy, plugin: &LoadedPlugin) {
    let command = plugin.resolved_command();
    let Some(executable) = command.first() else {
        return;
    };
    let executable = executable.to_string_lossy();
    let resolved = resolve_command(&executable, &plugin.dir);
    if let Some(path) = resolved {
        let runs_env = path.file_name().is_some_and(|name| name == "env");
        add_executable(policy, &path);
        if runs_env {
            if let Some(inner) = command.get(1) {
                if let Some(inner) = resolve_command(&inner.to_string_lossy(), &plugin.dir) {
                    add_executable(policy, &inner);
                }
            }
        }
    }
}

fn add_file_scope(
    policy: &mut SandboxPolicy,
    capabilities: &CapabilityRegistry,
    plugin: &str,
    capability: Capability,
    environment: &HostEnvironment,
    loaded: &LoadedPlugin,
) {
    let Some(scope) = capabilities.scope(plugin, capability) else {
        return;
    };
    let Scope::Paths(paths) = scope else {
        return;
    };
    let target = if capability == Capability::FsWrite {
        &mut policy.read_write
    } else {
        &mut policy.read_only
    };
    target.extend(
        paths
            .iter()
            .filter_map(|path| expand_path(path, loaded, environment))
            .filter_map(|path| existing(&path)),
    );
}

fn add_command_scope(
    policy: &mut SandboxPolicy,
    capabilities: &CapabilityRegistry,
    plugin: &LoadedPlugin,
) {
    let name = plugin.manifest.name.as_str();
    let Some(Scope::Commands(commands)) = capabilities.scope(name, Capability::ProcessSpawn) else {
        return;
    };
    for command in commands {
        if let Some(path) = resolve_command(command.as_str(), &plugin.dir) {
            add_executable(policy, &path);
        }
    }
}

fn add_executable(policy: &mut SandboxPolicy, path: &Path) {
    let Some(path) = existing(path) else {
        return;
    };
    if let Some(interpreter) = kawi_process::executable_interpreter(&path) {
        if let Some(interpreter) = existing(&interpreter) {
            policy.execute.push(interpreter);
        }
    }
    policy.execute.push(path);
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
