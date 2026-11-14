use std::collections::BTreeSet;

use crate::environment::is_path_variable;

use super::super::command;
use super::super::Capability;
use super::HostPort;

const MAX_EDITOR_COMMAND_LEN: usize = 64;

pub(crate) fn parse_request_scope(
    capability: Capability,
    value: &toml::Value,
) -> Result<super::Scope, String> {
    if !capability.is_scoped() {
        return match value {
            toml::Value::Boolean(true) => Ok(super::Scope::Unscoped),
            toml::Value::Boolean(false) => Err("false is not a request".to_string()),
            _ => Err("unscoped capabilities must be true".to_string()),
        };
    }
    let table = value
        .as_table()
        .ok_or_else(|| "scoped capabilities must be a table".to_string())?;
    parse_scoped_table(capability, table)
}

fn parse_scoped_table(
    capability: Capability,
    table: &toml::map::Map<String, toml::Value>,
) -> Result<super::Scope, String> {
    let key = scope_key(capability);
    if table.len() != 1 {
        return Err(format!("expected only the {key} key"));
    }
    let values = table
        .get(key)
        .ok_or_else(|| format!("missing {key} key"))?
        .as_array()
        .ok_or_else(|| format!("{key} must be an array"))?;
    if values.is_empty() {
        return Err(format!("{key} must not be empty"));
    }
    match capability {
        Capability::EditorBuffersWrite | Capability::FsRead | Capability::FsWrite => {
            parse_paths(values).map(super::Scope::Paths)
        }
        Capability::ProcessSpawn => parse_commands(values).map(super::Scope::Commands),
        Capability::EditorExecute => {
            parse_editor_commands(values).map(super::Scope::EditorCommands)
        }
        Capability::NetConnect => parse_hosts(values).map(super::Scope::Hosts),
        _ => Err("unscoped capability".to_string()),
    }
}

fn scope_key(capability: Capability) -> &'static str {
    match capability {
        Capability::EditorBuffersWrite | Capability::FsRead | Capability::FsWrite => "paths",
        Capability::ProcessSpawn | Capability::EditorExecute => "commands",
        Capability::NetConnect => "hosts",
        _ => "",
    }
}

fn parse_paths(values: &[toml::Value]) -> Result<Vec<String>, String> {
    let mut paths = BTreeSet::new();
    for value in values {
        let path = value
            .as_str()
            .ok_or_else(|| "paths must be strings".to_string())?;
        paths.insert(normalize_path(path)?);
    }
    Ok(paths.into_iter().collect())
}

fn parse_commands(values: &[toml::Value]) -> Result<Vec<String>, String> {
    let mut commands = BTreeSet::new();
    for value in values {
        let command = value
            .as_str()
            .ok_or_else(|| "commands must be strings".to_string())?;
        command::validate(command)?;
        commands.insert(command.to_string());
    }
    Ok(commands.into_iter().collect())
}

fn parse_editor_commands(values: &[toml::Value]) -> Result<Vec<String>, String> {
    let mut commands = BTreeSet::new();
    for value in values {
        let command = value
            .as_str()
            .ok_or_else(|| "commands must be strings".to_string())?;
        validate_editor_command(command)?;
        commands.insert(command.to_string());
    }
    Ok(commands.into_iter().collect())
}

fn validate_editor_command(command: &str) -> Result<(), String> {
    if command.is_empty() || command.len() > MAX_EDITOR_COMMAND_LEN {
        return Err(format!(
            "editor commands must be 1 to {MAX_EDITOR_COMMAND_LEN} characters"
        ));
    }
    if !command.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '-' | '_')
    }) {
        return Err(format!(
            "editor command {command} contains an invalid character"
        ));
    }
    if !command
        .starts_with(|character: char| character.is_ascii_lowercase() || character.is_ascii_digit())
    {
        return Err(format!(
            "editor command {command} must start with a letter or digit"
        ));
    }
    Ok(())
}

fn parse_hosts(values: &[toml::Value]) -> Result<Vec<String>, String> {
    let mut hosts = BTreeSet::new();
    for value in values {
        let host = normalize_host(
            value
                .as_str()
                .ok_or_else(|| "hosts must be strings".to_string())?,
        )?;
        hosts.insert(host);
    }
    Ok(hosts.into_iter().collect())
}

fn normalize_host(host: &str) -> Result<String, String> {
    HostPort::parse(host).map(|host| host.to_string())
}

fn normalize_path(path: &str) -> Result<String, String> {
    if path.is_empty() {
        return Err("paths must not be empty".to_string());
    }
    if path.split('/').any(|component| component == "..") {
        return Err("paths must not contain ..".to_string());
    }
    if let Some(variable) = path.strip_prefix('$') {
        let name = variable.split('/').next().unwrap_or_default();
        if !is_path_variable(name) {
            return Err(format!("unknown path variable: ${name}"));
        }
    } else if !path.starts_with('/') {
        return Err("paths must be absolute or start with a variable".to_string());
    }
    let mut parts = Vec::new();
    for component in path.split('/') {
        if component.is_empty() || component == "." {
            continue;
        }
        parts.push(component);
    }
    let normalized = if path.starts_with('/') {
        format!("/{}", parts.join("/"))
    } else {
        parts.join("/")
    };
    Ok(normalized)
}
