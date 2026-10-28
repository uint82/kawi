use std::fs;
use std::path::Path;
use std::time::Duration;

use toml_edit::{Document, Item};

use super::binding::{Binding, BindingSource, KeyContext};
use super::chord::{KeyChord, KeyCodeSpec, KeySequence};
use crate::command::CommandRegistry;
use crate::config::{line_at, ConfigWarning};

pub(crate) const DEFAULT_MAP_TIMEOUT_MS: u64 = 1000;
const MAX_MAP_TIMEOUT_MS: u64 = 60_000;

pub(crate) struct KeymapLoad {
    pub(crate) leader: Vec<KeyChord>,
    pub(crate) timeout: Duration,
    pub(crate) bindings: Vec<Binding>,
    pub(crate) warnings: Vec<ConfigWarning>,
}

impl Default for KeymapLoad {
    fn default() -> Self {
        Self {
            leader: super::default_leaders(),
            timeout: Duration::from_millis(DEFAULT_MAP_TIMEOUT_MS),
            bindings: Vec::new(),
            warnings: Vec::new(),
        }
    }
}

pub(crate) fn load(path: &Path, commands: &CommandRegistry) -> KeymapLoad {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return KeymapLoad::default(),
        Err(error) => {
            return KeymapLoad {
                warnings: vec![ConfigWarning::new(
                    None,
                    "keymaps",
                    error.to_string(),
                    "built-in defaults",
                )],
                ..KeymapLoad::default()
            }
        }
    };
    match Document::parse(text.as_str()) {
        Ok(document) => Loader::new(&text, commands).load(&document),
        Err(error) => KeymapLoad {
            warnings: vec![ConfigWarning::new(
                line_at(&text, error.span()),
                "keymaps",
                error.message(),
                "built-in defaults",
            )],
            ..KeymapLoad::default()
        },
    }
}

struct Loader<'a> {
    text: &'a str,
    commands: &'a CommandRegistry,
    load: KeymapLoad,
}

impl<'a> Loader<'a> {
    fn new(text: &'a str, commands: &'a CommandRegistry) -> Self {
        Self {
            text,
            commands,
            load: KeymapLoad::default(),
        }
    }

    fn load(mut self, document: &Document<&str>) -> KeymapLoad {
        self.load_leader(document.get("leader"));
        self.load_timeout(document.get("timeout"));
        for (key, item) in document.iter() {
            match key {
                "leader" | "timeout" => {}
                "normal" | "insert" | "visual" => self.load_mode(key, item),
                _ => self.warn(item, key, "ignored"),
            }
        }
        self.load
    }

    fn load_leader(&mut self, item: Option<&Item>) {
        let Some(item) = item else {
            return;
        };
        let Some(leaders) = parse_leaders(item) else {
            self.warn(item, "leader", "using default leaders");
            return;
        };
        if leaders.is_empty() {
            self.warn(item, "leader", "using default leaders");
            return;
        }
        self.load.leader = leaders;
    }

    fn load_timeout(&mut self, item: Option<&Item>) {
        let Some(item) = item else {
            return;
        };
        let Some(milliseconds) = item.as_integer() else {
            self.warn(item, "timeout", "using the default timeout");
            return;
        };
        let Ok(milliseconds) = u64::try_from(milliseconds) else {
            self.warn(item, "timeout", "using the default timeout");
            return;
        };
        if milliseconds > MAX_MAP_TIMEOUT_MS {
            self.warn(item, "timeout", "using the default timeout");
            return;
        }
        self.load.timeout = Duration::from_millis(milliseconds);
    }

    fn load_mode(&mut self, name: &str, item: &Item) {
        let Some(context) = KeyContext::parse(name) else {
            self.warn(item, name, "ignored");
            return;
        };
        let Some(table) = item.as_table_like() else {
            self.warn(item, name, "ignored, expected a table");
            return;
        };
        for (key, item) in table.iter() {
            self.load_binding(context, key, item);
        }
    }

    fn load_binding(&mut self, context: KeyContext, key: &str, item: &Item) {
        let warning_key = format!("{}.{}", context.name(), key);
        if let Some(table) = item.as_table_like() {
            for (field, field_item) in table.iter() {
                if matches!(field, "command" | "args" | "when" | "desc") {
                    continue;
                }
                self.warn(field_item, &warning_key, "ignored unknown field");
                return;
            }
        }
        let Some(expected) = parse_binding_value(item) else {
            self.warn(item, &warning_key, "ignored, expected a command or table");
            return;
        };
        let keys = match KeySequence::parse(key, &self.load.leader) {
            Ok(keys) => keys,
            Err(error) => {
                self.warn(item, &warning_key, error.to_string());
                return;
            }
        };
        if keys
            .first()
            .is_some_and(|chord| matches!(chord.code, KeyCodeSpec::Char(character) if character.is_ascii_digit() || character == '"'))
        {
            self.warn(item, &warning_key, "ignored, count and register prefixes are reserved");
            return;
        }
        let Some(command) = self.commands.find(&expected.command) else {
            self.warn(item, &warning_key, "ignored, unknown command");
            return;
        };
        self.load.bindings.push(
            Binding::new(command, context, keys, BindingSource::User)
                .with_args(expected.args)
                .with_when(expected.when)
                .with_description(expected.description),
        );
    }

    fn warn(&mut self, item: &Item, key: &str, fallback: impl Into<String>) {
        self.load.warnings.push(ConfigWarning::new(
            line_at(self.text, item.span()),
            key,
            item.to_string(),
            fallback,
        ));
    }
}

struct BindingValue {
    command: String,
    args: Option<toml::Value>,
    when: Option<String>,
    description: Option<String>,
}

fn parse_binding_value(item: &Item) -> Option<BindingValue> {
    if let Some(command) = item.as_str() {
        return Some(BindingValue {
            command: command.to_string(),
            args: None,
            when: None,
            description: None,
        });
    }
    let table = item.as_table_like()?;
    Some(BindingValue {
        command: table.get("command")?.as_str()?.to_string(),
        args: table.get("args").and_then(toml_value),
        when: table.get("when").and_then(Item::as_str).map(str::to_string),
        description: table.get("desc").and_then(Item::as_str).map(str::to_string),
    })
}

fn toml_value(item: &Item) -> Option<toml::Value> {
    let text = format!("value = {item}");
    toml::from_str::<toml::Table>(&text).ok()?.remove("value")
}

fn parse_leaders(item: &Item) -> Option<Vec<KeyChord>> {
    if let Some(text) = item.as_str() {
        return single_leader(text);
    }
    let array = item.as_array()?;
    let mut leaders = Vec::new();
    for value in array.iter() {
        let chord = single_leader(value.as_str()?)?.into_iter().next()?;
        if !leaders.contains(&chord) {
            leaders.push(chord);
        }
    }
    Some(leaders)
}

fn single_leader(text: &str) -> Option<Vec<KeyChord>> {
    let keys = KeySequence::parse(text, &[]).ok()?;
    (keys.len() == 1).then(|| keys.chords().to_vec())
}
