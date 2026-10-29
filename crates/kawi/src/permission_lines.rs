use kawi_plugin::{Capability, Scope};

pub(crate) fn grant_lines(
    plugin: &str,
    capability: Capability,
    scope: Option<&Scope>,
) -> [String; 2] {
    [
        format!("add to permissions.toml: [plugins.{plugin}.grant]"),
        format!("\"{}\" = {}", capability.name(), grant_value(scope)),
    ]
}

fn grant_value(scope: Option<&Scope>) -> String {
    match scope {
        None | Some(Scope::Unscoped) => "true".to_string(),
        Some(Scope::Paths(paths)) => format!("{{ paths = {paths:?} }}"),
        Some(Scope::Commands(commands)) => format!("{{ commands = {commands:?} }}"),
        Some(Scope::Hosts(hosts)) => format!("{{ hosts = {hosts:?} }}"),
    }
}
