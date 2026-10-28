use anyhow::Result;

use crate::keymap::{Binding, BindingSource, KeyCodeSpec, KeyContext};

use super::App;

impl App {
    pub(super) fn handle_map_command(&mut self, text: &str) -> Result<Option<bool>> {
        let text = text.trim();
        let (name, rest) = split_command(text);
        match name {
            "nmap" => self.add_mapping_command(KeyContext::Normal, rest),
            "imap" => self.add_mapping_command(KeyContext::Insert, rest),
            "vmap" => self.add_mapping_command(KeyContext::Visual, rest),
            "nunmap" => self.remove_mapping_command(KeyContext::Normal, rest),
            "iunmap" => self.remove_mapping_command(KeyContext::Insert, rest),
            "vunmap" => self.remove_mapping_command(KeyContext::Visual, rest),
            "map" => self.list_mappings(rest),
            "commands" => self.list_commands(),
            _ => Ok(None),
        }
    }

    fn add_mapping_command(&mut self, mode: KeyContext, rest: &str) -> Result<Option<bool>> {
        let (keys, command) = split_command(rest);
        if keys.is_empty() || command.is_empty() {
            self.show_message("E474: Expected a key sequence and command".to_string());
            return Ok(Some(false));
        }
        let (command, args) = split_mapping_target(command);
        let sequences = match crate::keymap::config::expand_sequences(keys, &self.leaders) {
            Ok(sequences) => sequences,
            Err(error) => {
                self.show_message(format!("E474: {error}"));
                return Ok(Some(false));
            }
        };
        if sequences.iter().any(|keys| {
            keys.first().is_some_and(|chord| {
                matches!(chord.code, KeyCodeSpec::Char(character) if character.is_ascii_digit() || character == '"')
            })
        }) {
            self.show_message("E474: Count and register prefixes are reserved".to_string());
            return Ok(Some(false));
        }
        let Some(command) = self.commands.find(command) else {
            self.show_message(format!("E492: Unknown command: {command}"));
            return Ok(Some(false));
        };
        for keys in sequences {
            let binding = Binding::new(command, mode, keys.clone(), BindingSource::User)
                .with_args(args.clone());
            self.keymaps.bind(binding);
            self.show_message(format!("mapped {} {}", mode.name(), keys));
        }
        Ok(Some(false))
    }

    fn remove_mapping_command(&mut self, mode: KeyContext, rest: &str) -> Result<Option<bool>> {
        let keys = rest.trim();
        if keys.is_empty() {
            self.show_message("E474: Expected a key sequence".to_string());
            return Ok(Some(false));
        }
        let sequences = match crate::keymap::config::expand_sequences(keys, &self.leaders) {
            Ok(sequences) => sequences,
            Err(error) => {
                self.show_message(format!("E474: {error}"));
                return Ok(Some(false));
            }
        };
        for keys in sequences {
            if !self.keymaps.has_binding(mode, &keys) {
                self.show_message(format!("E31: No such mapping: {} {}", mode.name(), keys));
                continue;
            }
            self.keymaps.unbind(mode, &keys);
            self.show_message(format!("unmapped {} {}", mode.name(), keys));
        }
        Ok(Some(false))
    }

    fn list_mappings(&mut self, rest: &str) -> Result<Option<bool>> {
        let rest = rest.trim();
        if rest == "reload" {
            return self.reload_keymaps();
        }
        let filter = match rest {
            "" => None,
            name => match KeyContext::parse(name) {
                Some(context) => Some(context),
                None => {
                    self.show_message("E474: Expected map reload or a mode name".to_string());
                    return Ok(Some(false));
                }
            },
        };
        let lines = self.mapping_lines(filter);
        if lines.is_empty() {
            self.show_message("No key mappings".to_string());
        } else {
            self.show_pager(lines);
        }
        Ok(Some(false))
    }

    fn reload_keymaps(&mut self) -> Result<Option<bool>> {
        let Some(path) = self.keymaps_path.clone() else {
            self.show_message("E484: No keymap file loaded".to_string());
            return Ok(Some(false));
        };
        let loaded = crate::keymap::config::load(&path, &self.commands);
        let warnings: Vec<String> = loaded
            .warnings
            .iter()
            .map(|warning| warning.format(&path))
            .collect();
        self.apply_keymaps(loaded);
        if warnings.is_empty() {
            self.show_message("Keymaps reloaded".to_string());
        } else {
            self.show_pager(warnings);
        }
        Ok(Some(false))
    }

    fn list_commands(&mut self) -> Result<Option<bool>> {
        let mut lines: Vec<String> = self
            .commands
            .specs()
            .map(|spec| {
                let aliases = if spec.aliases.is_empty() {
                    String::new()
                } else {
                    format!(" ({})", spec.aliases.join(", "))
                };
                format!("{:<28} {}{}", spec.name, spec.description, aliases)
            })
            .collect();
        lines.sort();
        self.show_pager(lines);
        Ok(Some(false))
    }

    fn mapping_lines(&self, filter: Option<KeyContext>) -> Vec<String> {
        let mut rows: Vec<(String, String, String, String)> = self
            .keymaps
            .bindings()
            .iter()
            .filter(|binding| filter.is_none_or(|context| binding.mode == context))
            .map(|binding| {
                let command = self
                    .commands
                    .spec(binding.command)
                    .map(|spec| spec.name.as_str())
                    .unwrap_or("unknown");
                let description = binding
                    .description
                    .as_deref()
                    .or_else(|| {
                        self.commands
                            .spec(binding.command)
                            .map(|spec| spec.description.as_str())
                    })
                    .unwrap_or("");
                (
                    binding.mode.name().to_string(),
                    binding.keys.to_string(),
                    command.to_string(),
                    description.to_string(),
                )
            })
            .collect();
        rows.sort();
        rows.into_iter()
            .map(|(mode, keys, command, description)| {
                format!("{mode:<7} {keys:<24} {command:<28} {description}")
            })
            .collect()
    }
}

fn split_command(text: &str) -> (&str, &str) {
    let text = text.trim();
    match text.split_once(char::is_whitespace) {
        Some((name, rest)) => (name, rest.trim()),
        None => (text, ""),
    }
}

fn split_mapping_target(text: &str) -> (&str, Option<toml::Value>) {
    match text.split_once(char::is_whitespace) {
        Some((command, arguments)) if command == "keys.send" => (
            command,
            Some(toml::Value::String(arguments.trim().to_string())),
        ),
        Some((command, arguments)) if !arguments.trim().is_empty() => (
            command,
            Some(toml::Value::String(arguments.trim().to_string())),
        ),
        _ => (text.trim(), None),
    }
}

#[cfg(test)]
mod tests;
