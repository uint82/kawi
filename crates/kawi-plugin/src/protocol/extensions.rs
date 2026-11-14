use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const MAX_RUNTIME_COMMANDS_PER_PLUGIN: usize = 256;
pub const MAX_RUNTIME_COMMAND_ID_BYTES: usize = 64;
pub const MAX_RUNTIME_COMMAND_TITLE_BYTES: usize = 256;
pub const MAX_RUNTIME_COMMAND_DESCRIPTION_BYTES: usize = 1024;
pub const MAX_RUNTIME_COMMAND_ALIASES: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandRegisterParams {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandUnregisterParams {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisteredCommand {
    pub id: String,
    pub name: String,
    pub title: String,
    pub description: String,
    pub aliases: Vec<String>,
}

pub fn validate_command_spec(spec: &CommandRegisterParams) -> Result<(), String> {
    validate_command_id(&spec.id)?;
    if spec.title.trim().is_empty() {
        return Err("command title must not be empty".to_string());
    }
    if spec.title.len() > MAX_RUNTIME_COMMAND_TITLE_BYTES {
        return Err(format!(
            "command title must not exceed {MAX_RUNTIME_COMMAND_TITLE_BYTES} bytes"
        ));
    }
    if let Some(description) = &spec.description {
        if description.len() > MAX_RUNTIME_COMMAND_DESCRIPTION_BYTES {
            return Err(format!(
                "command description must not exceed {MAX_RUNTIME_COMMAND_DESCRIPTION_BYTES} bytes"
            ));
        }
    }
    if spec.aliases.len() > MAX_RUNTIME_COMMAND_ALIASES {
        return Err(format!(
            "command aliases must not exceed {MAX_RUNTIME_COMMAND_ALIASES} entries"
        ));
    }
    let mut seen = BTreeSet::new();
    for alias in &spec.aliases {
        validate_command_id(alias)?;
        if alias == &spec.id {
            return Err("command alias must differ from its id".to_string());
        }
        if !seen.insert(alias) {
            return Err(format!("duplicate command alias: {alias}"));
        }
    }
    Ok(())
}

pub fn validate_command_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > MAX_RUNTIME_COMMAND_ID_BYTES {
        return Err(format!(
            "command id must be 1 to {MAX_RUNTIME_COMMAND_ID_BYTES} characters"
        ));
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

pub const MAX_RUNTIME_KEYMAPS_PER_PLUGIN: usize = 256;
pub const MAX_RUNTIME_KEYMAP_KEYS_BYTES: usize = 256;
pub const MAX_RUNTIME_KEYMAP_COMMAND_BYTES: usize = 128;
pub const MAX_RUNTIME_KEYMAP_DESCRIPTION_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum KeyMode {
    Normal,
    Insert,
    Visual,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeymapSetParams {
    pub mode: KeyMode,
    pub keys: String,
    pub command: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeymapRemoveParams {
    pub mode: KeyMode,
    pub keys: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RegisteredKeymap {
    pub mode: KeyMode,
    pub keys: String,
    pub command: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub fn validate_keymap_spec(spec: &KeymapSetParams) -> Result<(), String> {
    validate_keymap_keys(&spec.keys)?;
    if spec.command.is_empty() || spec.command.len() > MAX_RUNTIME_KEYMAP_COMMAND_BYTES {
        return Err(format!(
            "keymap command must be 1 to {MAX_RUNTIME_KEYMAP_COMMAND_BYTES} bytes"
        ));
    }
    if spec.command.chars().any(char::is_control) {
        return Err("keymap command contains a control character".to_string());
    }
    if let Some(description) = &spec.description {
        if description.len() > MAX_RUNTIME_KEYMAP_DESCRIPTION_BYTES {
            return Err(format!(
                "keymap description must not exceed {MAX_RUNTIME_KEYMAP_DESCRIPTION_BYTES} bytes"
            ));
        }
    }
    Ok(())
}

pub fn validate_keymap_keys(keys: &str) -> Result<(), String> {
    if keys.trim().is_empty() {
        return Err("keymap keys must not be empty".to_string());
    }
    if keys.len() > MAX_RUNTIME_KEYMAP_KEYS_BYTES {
        return Err(format!(
            "keymap keys must not exceed {MAX_RUNTIME_KEYMAP_KEYS_BYTES} bytes"
        ));
    }
    if keys.chars().any(char::is_control) {
        return Err("keymap keys contain a control character".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_spec() -> CommandRegisterParams {
        CommandRegisterParams {
            id: "greet".to_string(),
            title: "Greet".to_string(),
            description: Some("Say hello".to_string()),
            aliases: vec!["hello".to_string()],
        }
    }

    #[test]
    fn accepts_a_valid_command_spec() {
        assert!(validate_command_spec(&valid_spec()).is_ok());
    }

    #[test]
    fn rejects_invalid_command_ids_and_aliases() {
        for id in ["", "Greet", "-greet", "greet!", "a/b"] {
            assert!(validate_command_id(id).is_err(), "{id}");
        }
        let mut invalid = valid_spec();
        invalid.id = "Greet".to_string();
        assert!(validate_command_spec(&invalid).is_err());
        let mut invalid = valid_spec();
        invalid.aliases = vec!["_hello".to_string()];
        assert!(validate_command_spec(&invalid).is_err());
    }

    #[test]
    fn rejects_empty_and_oversized_titles() {
        let mut invalid = valid_spec();
        invalid.title = "  ".to_string();
        assert!(validate_command_spec(&invalid).is_err());
        invalid.title = "a".repeat(MAX_RUNTIME_COMMAND_TITLE_BYTES + 1);
        assert!(validate_command_spec(&invalid).is_err());
    }

    #[test]
    fn rejects_oversized_descriptions_and_aliases() {
        let mut invalid = valid_spec();
        invalid.description = Some("a".repeat(MAX_RUNTIME_COMMAND_DESCRIPTION_BYTES + 1));
        assert!(validate_command_spec(&invalid).is_err());
        let mut invalid = valid_spec();
        invalid.aliases = (0..=MAX_RUNTIME_COMMAND_ALIASES)
            .map(|index| format!("alias{index}"))
            .collect();
        assert!(validate_command_spec(&invalid).is_err());
    }

    #[test]
    fn rejects_duplicate_and_self_referencing_aliases() {
        let mut invalid = valid_spec();
        invalid.aliases = vec!["hello".to_string(), "hello".to_string()];
        assert!(validate_command_spec(&invalid).is_err());
        let mut invalid = valid_spec();
        invalid.aliases = vec![invalid.id.clone()];
        assert!(validate_command_spec(&invalid).is_err());
    }
}

#[cfg(test)]
mod keymap_tests {
    use super::*;

    fn valid_keymap() -> KeymapSetParams {
        KeymapSetParams {
            mode: KeyMode::Normal,
            keys: "<leader>pg".to_string(),
            command: "demo.greet".to_string(),
            description: Some("Greet".to_string()),
        }
    }

    #[test]
    fn accepts_a_valid_keymap_spec() {
        assert!(validate_keymap_spec(&valid_keymap()).is_ok());
    }

    #[test]
    fn rejects_empty_and_oversized_keys() {
        let mut invalid = valid_keymap();
        invalid.keys = "  ".to_string();
        assert!(validate_keymap_spec(&invalid).is_err());
        invalid.keys = "k".repeat(MAX_RUNTIME_KEYMAP_KEYS_BYTES + 1);
        assert!(validate_keymap_spec(&invalid).is_err());
        invalid.keys = "a\nb".to_string();
        assert!(validate_keymap_spec(&invalid).is_err());
    }

    #[test]
    fn rejects_invalid_commands() {
        let mut invalid = valid_keymap();
        invalid.command = String::new();
        assert!(validate_keymap_spec(&invalid).is_err());
        invalid.command = "a".repeat(MAX_RUNTIME_KEYMAP_COMMAND_BYTES + 1);
        assert!(validate_keymap_spec(&invalid).is_err());
        invalid.command = "bad\u{0000}".to_string();
        assert!(validate_keymap_spec(&invalid).is_err());
    }

    #[test]
    fn rejects_oversized_descriptions() {
        let mut invalid = valid_keymap();
        invalid.description = Some("a".repeat(MAX_RUNTIME_KEYMAP_DESCRIPTION_BYTES + 1));
        assert!(validate_keymap_spec(&invalid).is_err());
    }
}
