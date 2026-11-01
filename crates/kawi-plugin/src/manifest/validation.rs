use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::runtime;
use super::{ActivationEvent, PluginCommand, PluginManifest};
use crate::capability::Capability;
use crate::protocol::PROTOCOL_VERSION;

const MIN_TIMEOUT_MS: u64 = 100;
const MAX_TIMEOUT_MS: u64 = 60_000;
const MAX_NAME_LEN: usize = 64;

pub(super) fn validate_manifest(
    manifest: &mut PluginManifest,
    dir: &std::path::Path,
) -> Result<runtime::Artifact, String> {
    validate_name(&manifest.name)?;
    if manifest.version.trim().is_empty() {
        return Err("version must not be empty".to_string());
    }
    if manifest.protocol != PROTOCOL_VERSION {
        return Err(format!(
            "unsupported protocol {}, expected {}",
            manifest.protocol, PROTOCOL_VERSION
        ));
    }
    let artifact = runtime::validate(manifest, dir)?;
    validate_timeout(manifest.timeout_ms)?;
    super::settings::validate(&mut manifest.settings)?;
    validate_commands(&manifest.commands)?;
    validate_subscriptions(manifest)?;
    validate_activation(&manifest.activation_events)?;
    Ok(artifact)
}

fn validate_subscriptions(manifest: &PluginManifest) -> Result<(), String> {
    let mut names = HashSet::new();
    for event in &manifest.subscribes {
        if event.trim().is_empty() {
            return Err("subscription names must not be empty".to_string());
        }
        if !names.insert(event.as_str()) {
            return Err(format!("duplicate subscription: {event}"));
        }
    }
    for (event, debounce_ms) in &manifest.debounce {
        if !names.contains(event.as_str()) {
            return Err(format!("debounce set for unsubscribed event: {event}"));
        }
        if *debounce_ms > MAX_TIMEOUT_MS {
            return Err(format!(
                "debounce for {event} must not exceed {MAX_TIMEOUT_MS}"
            ));
        }
    }
    Ok(())
}

fn validate_activation(events: &[ActivationEvent]) -> Result<(), String> {
    for event in events {
        match event.event.as_str() {
            "on_startup" | "on_command" => {
                if !event.filetypes.is_empty() || !event.globs.is_empty() {
                    return Err(format!("{} does not accept filters", event.event));
                }
            }
            "on_file_open" => {
                if event.filetypes.iter().any(|value| value.trim().is_empty()) {
                    return Err("activation filetypes must not be empty".to_string());
                }
                if event.globs.iter().any(|value| value.trim().is_empty()) {
                    return Err("activation globs must not be empty".to_string());
                }
            }
            other => return Err(format!("unknown activation event: {other}")),
        }
    }
    Ok(())
}

pub(super) fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > MAX_NAME_LEN {
        return Err(format!("name must be 1 to {MAX_NAME_LEN} characters"));
    }
    if !name.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '-' | '_')
    }) {
        return Err(
            "name may contain only lowercase letters, digits, hyphens, and underscores".to_string(),
        );
    }
    if !name
        .starts_with(|character: char| character.is_ascii_lowercase() || character.is_ascii_digit())
    {
        return Err("name must start with a letter or digit".to_string());
    }
    Ok(())
}

pub(super) fn validate_command(command: &[String]) -> Result<(), String> {
    if command.is_empty() {
        return Err("command must not be empty".to_string());
    }
    if command.iter().any(|argument| argument.is_empty()) {
        return Err("command arguments must not be empty".to_string());
    }
    Ok(())
}

fn validate_timeout(timeout_ms: u64) -> Result<(), String> {
    if !(MIN_TIMEOUT_MS..=MAX_TIMEOUT_MS).contains(&timeout_ms) {
        return Err(format!(
            "timeout_ms must be between {MIN_TIMEOUT_MS} and {MAX_TIMEOUT_MS}"
        ));
    }
    Ok(())
}

fn validate_commands(commands: &[PluginCommand]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for command in commands {
        validate_command_id(&command.id)?;
        if command.title.trim().is_empty() {
            return Err(format!("command {} has an empty title", command.id));
        }
        if !ids.insert(command.id.as_str()) {
            return Err(format!("duplicate command id: {}", command.id));
        }
        validate_keybindings(&command.keybindings)?;
    }
    command_capabilities(commands)?;
    Ok(())
}

pub(super) fn command_capabilities(
    commands: &[PluginCommand],
) -> Result<BTreeMap<String, BTreeSet<Capability>>, String> {
    let mut gates = BTreeMap::new();
    for command in commands {
        let mut capabilities = BTreeSet::new();
        for name in &command.capabilities {
            let Some(capability) = Capability::parse(name) else {
                return Err(format!(
                    "command {} declares unknown capability: {name}",
                    command.id
                ));
            };
            if !capabilities.insert(capability) {
                return Err(format!(
                    "command {} declares duplicate capability: {name}",
                    command.id
                ));
            }
        }
        if !capabilities.is_empty() {
            gates.insert(command.id.clone(), capabilities);
        }
    }
    Ok(gates)
}

fn validate_command_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > MAX_NAME_LEN {
        return Err(format!("command id must be 1 to {MAX_NAME_LEN} characters"));
    }
    if !id.chars().all(|character| {
        character.is_ascii_lowercase()
            || character.is_ascii_digit()
            || matches!(character, '.' | '-' | '_')
    }) {
        return Err(format!("command id {id} contains an invalid character"));
    }
    if !id
        .starts_with(|character: char| character.is_ascii_lowercase() || character.is_ascii_digit())
    {
        return Err(format!("command id {id} must start with a letter or digit"));
    }
    Ok(())
}

fn validate_keybindings(bindings: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    for (mode, sequences) in bindings {
        if !matches!(mode.as_str(), "normal" | "insert" | "visual") {
            return Err(format!("unknown keybinding mode: {mode}"));
        }
        if sequences.is_empty() {
            return Err(format!("keybinding mode {mode} must not be empty"));
        }
        if sequences.iter().any(|sequence| sequence.trim().is_empty()) {
            return Err(format!("keybinding mode {mode} contains an empty sequence"));
        }
    }
    Ok(())
}
