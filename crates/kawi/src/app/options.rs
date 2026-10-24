use anyhow::Result;
use kawi_editor::{SetValue, WindowOptions};

use super::App;
use crate::keymap::{flavor_from_name, ActiveKeymap, Flavor};
use crate::options::{self, OptionError, OptionInput, OptionSpec, OptionValue};
use kawi_tui::theme::Theme;

impl App {
    pub(super) fn handle_set(&mut self, name: &str, action: SetValue) -> Result<bool> {
        let Some(spec) = options::lookup(name) else {
            self.show_message(format!("E518: Unknown option: {name}"));
            return Ok(false);
        };
        match action {
            SetValue::Query => self.show_option(spec),
            SetValue::Reset => self.apply_option(spec, spec.default_value())?,
            SetValue::Toggle => self.toggle_option(spec)?,
            SetValue::Negate => self.negate_option(spec)?,
            SetValue::Assign(None) => self.assign_option(spec, None)?,
            SetValue::Assign(Some(value)) => self.assign_option(spec, Some(&value))?,
        }
        Ok(false)
    }

    fn assign_option(&mut self, spec: &'static OptionSpec, value: Option<&str>) -> Result<()> {
        let input = match (spec.kind, value) {
            (options::OptionKind::Bool, None) => OptionInput::Bool(true),
            (_, None) => {
                self.show_message(format!("E521: Argument required: {}", spec.name));
                return Ok(());
            }
            (_, Some(value)) => OptionInput::Text(value),
        };
        self.resolve_and_apply(spec, input)
    }

    fn toggle_option(&mut self, spec: &'static OptionSpec) -> Result<()> {
        let Some(current) = self.option_value(spec).as_bool() else {
            self.show_message(format!("E474: Invalid argument: {}!", spec.name));
            return Ok(());
        };
        self.apply_option(spec, OptionValue::Bool(!current))
    }

    fn negate_option(&mut self, spec: &'static OptionSpec) -> Result<()> {
        if !matches!(spec.kind, options::OptionKind::Bool) {
            self.show_message(format!("E474: Invalid argument: no{}", spec.name));
            return Ok(());
        }
        self.apply_option(spec, OptionValue::Bool(false))
    }

    fn resolve_and_apply(
        &mut self,
        spec: &'static OptionSpec,
        input: OptionInput<'_>,
    ) -> Result<()> {
        if spec.name == "theme" {
            if let OptionInput::Text(value) = input {
                return self.apply_option(spec, OptionValue::Text(value.to_string()));
            }
        }
        let validated = {
            let context = options::OptionContext {
                theme_dir: self.theme_dir.as_deref(),
            };
            options::validate_spec(spec, input, &context)
        };
        match validated {
            Ok(value) => self.apply_option(spec, value),
            Err(error) => {
                self.show_message(option_error_message(spec, &error));
                Ok(())
            }
        }
    }

    pub(super) fn apply_option(
        &mut self,
        spec: &'static OptionSpec,
        value: OptionValue,
    ) -> Result<()> {
        match spec.name {
            "keymap" => self.apply_keymap_value(&value)?,
            "theme" => self.apply_theme_value(&value)?,
            "history" => self
                .editor
                .set_history_limit(value.as_int().unwrap_or_default()),
            "shiftwidth" => self
                .editor
                .set_shiftwidth(value.as_int().unwrap_or_default()),
            "expandtab" => self
                .editor
                .set_expandtab(value.as_bool().unwrap_or_default()),
            "number" => self.editor.set_number(value.as_bool().unwrap_or_default()),
            "relativenumber" => self
                .editor
                .set_relativenumber(value.as_bool().unwrap_or_default()),
            "numberwidth" => self
                .editor
                .set_numberwidth(value.as_int().unwrap_or_default()),
            "ignorecase" => self
                .editor
                .set_ignorecase(value.as_bool().unwrap_or_default()),
            "smartcase" => self
                .editor
                .set_smartcase(value.as_bool().unwrap_or_default()),
            "autoindent" => self
                .editor
                .set_autoindent(value.as_bool().unwrap_or_default()),
            _ => {}
        }
        if spec.scope == options::OptionScope::Window {
            self.sync_active_window_options();
        }
        Ok(())
    }

    fn apply_keymap_value(&mut self, value: &OptionValue) -> Result<()> {
        let name = value.as_text().unwrap_or_default();
        match flavor_from_name(name) {
            Some(flavor) => self.apply_flavor(flavor),
            None => {
                self.show_message(format!("Unknown keymap: {name}"));
                Ok(())
            }
        }
    }

    fn apply_theme_value(&mut self, value: &OptionValue) -> Result<()> {
        let name = value.as_text().unwrap_or_default();
        let loaded = match self.theme_dir.as_deref() {
            Some(theme_dir) => Theme::load(name, theme_dir),
            None => match Theme::named(name) {
                Some(theme) => Ok(theme),
                None => {
                    self.show_message(format!("Unknown theme: {name}"));
                    return Ok(());
                }
            },
        };
        match loaded {
            Ok(theme) => {
                self.theme = theme;
                Ok(())
            }
            Err(error) if error.is_not_found() => {
                self.show_message(format!("Unknown theme: {name}"));
                Ok(())
            }
            Err(error) => {
                self.show_message(format!("Invalid theme {name}: {error}"));
                Ok(())
            }
        }
    }

    pub(super) fn apply_cli_flavor(&mut self, flavor: Flavor) -> Result<()> {
        let spec = options::lookup("keymap").expect("keymap is registered");
        self.resolve_and_apply(spec, OptionInput::Text(flavor.name()))
    }

    pub(super) fn apply_flavor(&mut self, flavor: Flavor) -> Result<()> {
        self.flavor = flavor;
        self.keymap = ActiveKeymap::for_flavor(flavor);
        if flavor == Flavor::Vim {
            self.exit_visual_if_active();
        }
        self.reveal_cursor();
        Ok(())
    }

    pub(super) fn apply_window_options(&mut self, options: WindowOptions) {
        self.editor.set_window_options(options);
        self.window_defaults = options;
        self.windows.set_all_options(options);
        self.sync_active_window_options();
    }

    fn show_option(&mut self, spec: &'static OptionSpec) {
        let value = self.option_value(spec);
        self.show_message(option_query(spec, &value));
    }

    fn option_value(&self, spec: &'static OptionSpec) -> OptionValue {
        match spec.name {
            "keymap" => OptionValue::Text(self.flavor.name().to_string()),
            "theme" => OptionValue::Text(self.theme.name.to_string()),
            "history" => OptionValue::Int(self.editor.history_limit()),
            "shiftwidth" => OptionValue::Int(self.editor.shiftwidth()),
            "expandtab" => OptionValue::Bool(self.editor.expandtab()),
            "number" => OptionValue::Bool(self.editor.number()),
            "relativenumber" => OptionValue::Bool(self.editor.relativenumber()),
            "numberwidth" => OptionValue::Int(self.editor.numberwidth()),
            "ignorecase" => OptionValue::Bool(self.editor.ignorecase()),
            "smartcase" => OptionValue::Bool(self.editor.smartcase()),
            "autoindent" => OptionValue::Bool(self.editor.autoindent()),
            _ => spec.default_value(),
        }
    }
}

fn option_query(spec: &'static OptionSpec, value: &OptionValue) -> String {
    match value {
        OptionValue::Bool(true) => spec.name.to_string(),
        OptionValue::Bool(false) => format!("no{}", spec.name),
        OptionValue::Int(value) => format!("{}={value}", spec.name),
        OptionValue::Text(value) => format!("{}={value}", spec.name),
    }
}

fn option_error_message(spec: &'static OptionSpec, error: &OptionError) -> String {
    match error {
        OptionError::InvalidType { expected, value } => {
            format!(
                "E474: Invalid argument: {}={value}, expected {expected}",
                spec.name
            )
        }
        OptionError::OutOfRange { value, .. } => {
            if value < &1 && spec.name == "numberwidth" {
                format!(
                    "E487: Argument must be positive: {name}={value}",
                    name = spec.name
                )
            } else {
                format!("E474: Invalid argument: {}={value}", spec.name)
            }
        }
        OptionError::InvalidValue { value } => match spec.name {
            "keymap" => format!("Unknown keymap: {value}"),
            "theme" => format!("Unknown theme: {value}"),
            _ => format!("E474: Invalid argument: {}={value}", spec.name),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;
    use std::fs;

    fn app() -> App {
        App::new(Buffer::from_string("alpha\nbeta\n"), Flavor::Vim)
    }

    fn execute(app: &mut App, text: &str) {
        app.run_command(kawi_editor::parse(text)).unwrap();
    }

    #[test]
    fn set_supports_boolean_forms() {
        let mut app = app();
        execute(&mut app, "set nonumber");
        assert!(!app.editor.number());
        execute(&mut app, "set number");
        assert!(app.editor.number());
        execute(&mut app, "set number!");
        assert!(!app.editor.number());
        execute(&mut app, "set number?");
        assert_eq!(app.last_message(), Some("nonumber"));
        execute(&mut app, "set number&");
        assert!(app.editor.number());
        execute(&mut app, "set number=false");
        assert!(!app.editor.number());
    }

    #[test]
    fn set_supports_integer_aliases_and_reset() {
        let mut app = app();
        execute(&mut app, "set sw=2");
        assert_eq!(app.editor.shiftwidth(), 2);
        execute(&mut app, "set sw?");
        assert_eq!(app.last_message(), Some("shiftwidth=2"));
        execute(&mut app, "set shiftwidth&");
        assert_eq!(app.editor.shiftwidth(), 8);
    }

    #[test]
    fn set_queries_dynamic_enums() {
        let mut app = app();
        execute(&mut app, "set theme gruvbox-dark");
        execute(&mut app, "set theme?");
        assert_eq!(app.last_message(), Some("theme=gruvbox-dark"));
        execute(&mut app, "set keymap helix");
        execute(&mut app, "set keymap?");
        assert_eq!(app.last_message(), Some("keymap=helix"));
    }

    #[test]
    fn set_reports_unknown_and_invalid_options() {
        let mut app = app();
        execute(&mut app, "set wat");
        assert_eq!(app.last_message(), Some("E518: Unknown option: wat"));
        execute(&mut app, "set numberwidth=0");
        assert_eq!(
            app.last_message(),
            Some("E487: Argument must be positive: numberwidth=0")
        );
        execute(&mut app, "set theme=nope");
        assert_eq!(app.last_message(), Some("Unknown theme: nope"));
        execute(&mut app, "set noexpandtab");
        assert!(!app.editor.expandtab());
        execute(&mut app, "set notheme");
        assert_eq!(app.last_message(), Some("E474: Invalid argument: notheme"));
    }

    #[test]
    fn buffer_options_travel_with_their_buffer() {
        let mut app = app();
        let first = app.editor.buffer_number();
        execute(&mut app, "set sw=2");
        execute(&mut app, "set noexpandtab");
        execute(&mut app, "set noautoindent");
        app.editor.open_empty_buffer();
        assert_eq!(app.editor.shiftwidth(), 8);
        assert!(!app.editor.expandtab());
        assert!(app.editor.autoindent());
        assert!(app.editor.buffer_select(first));
        assert_eq!(app.editor.shiftwidth(), 2);
        assert!(!app.editor.expandtab());
        assert!(!app.editor.autoindent());
    }

    #[test]
    fn set_loads_custom_theme_files() {
        let dir = std::env::temp_dir().join(format!(
            "kawi_custom_theme_set_{}_{}",
            std::process::id(),
            "valid"
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("personal.toml"),
            crate::test_support::custom_theme(),
        )
        .unwrap();
        let mut app = app();
        app.theme_dir = Some(dir.clone());
        execute(&mut app, "set theme personal");
        assert_eq!(app.theme_name(), "personal");
        assert_eq!(app.last_message(), None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn set_reports_invalid_custom_theme_files() {
        let dir = std::env::temp_dir().join(format!(
            "kawi_custom_theme_set_{}_{}",
            std::process::id(),
            "invalid"
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("personal.toml"), "[base]\n").unwrap();
        let mut app = app();
        app.theme_dir = Some(dir.clone());
        execute(&mut app, "set theme personal");
        assert_eq!(app.theme_name(), "default");
        assert!(app
            .last_message()
            .is_some_and(|message| message.starts_with("Invalid theme personal:")));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn window_options_are_per_window() {
        let mut app = app();
        let first = app.windows.active();
        app.split_window(false, None).unwrap();
        let second = app.windows.active();
        execute(&mut app, "set nonumber");
        assert!(!app.editor.number());
        assert!(app.focus_window(first));
        assert!(app.editor.number());
        app.sync_active_window();
        assert!(app.focus_window(second));
        assert!(!app.editor.number());
    }
}
