use anyhow::Result;
use kawi_editor::{FoldMethod, SetValue, WindowOptions};

use super::App;
use crate::keymap::{flavor_from_name, ActiveKeymap, Flavor};
use crate::options::{
    self, AutosaveMode, OptionContext, OptionError, OptionInput, OptionSpec, OptionValue,
};
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
            "undo_persist" => self.undo.set_enabled(value.as_bool().unwrap_or(true)),
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
            "signcolumn" => {
                self.signcolumn = value
                    .as_text()
                    .and_then(options::SignColumnMode::parse)
                    .unwrap_or_default();
                self.update_signcolumn_width();
            }
            "git_signs" => {
                self.git_signs = value.as_bool().unwrap_or(false);
                self.update_signcolumn_width();
            }
            "ignorecase" => self
                .editor
                .set_ignorecase(value.as_bool().unwrap_or_default()),
            "smartcase" => self
                .editor
                .set_smartcase(value.as_bool().unwrap_or_default()),
            "autoindent" => self
                .editor
                .set_autoindent(value.as_bool().unwrap_or_default()),
            "foldmethod" => self.editor.set_fold_method(
                value
                    .as_text()
                    .and_then(FoldMethod::parse)
                    .unwrap_or_default(),
            ),
            "foldcolumn" => self
                .editor
                .set_fold_column(value.as_int().unwrap_or_default()),
            "auto_pairs" => self
                .editor
                .set_auto_pairs(value.as_bool().unwrap_or_default()),
            "match_brackets" => self
                .editor
                .set_match_brackets(value.as_bool().unwrap_or_default()),
            "format_on_save" => self.format_on_save = value.as_bool().unwrap_or_default(),
            "autosave" => self.autosave.set_mode(
                value
                    .as_text()
                    .and_then(AutosaveMode::parse)
                    .unwrap_or_default(),
            ),
            "autosave_delay" => self
                .autosave
                .set_delay_millis(value.as_int().unwrap_or_default()),
            "auto_completion" => {
                self.completion_options.auto = value.as_bool().unwrap_or_default();
                if !self.completion_options.auto {
                    self.completion_auto.cancel();
                }
            }
            "signature_help" => {
                self.completion_options.signature = value.as_bool().unwrap_or(true);
                if !self.completion_options.signature {
                    self.signature = None;
                    self.lsp.cancel_signature();
                }
            }
            "completion_docs_up" => {
                self.apply_completion_key(&value, true);
            }
            "completion_docs_down" => {
                self.apply_completion_key(&value, false);
            }
            "hover_max_width" => {
                self.overlay_options.hover_max_width = value.as_int().unwrap_or_default();
            }
            "completion_docs_max_width" => {
                self.overlay_options.completion_docs_max_width = value.as_int().unwrap_or_default();
            }
            "diagnostic_popup_max_width" => {
                self.overlay_options.diagnostic_popup_max_width =
                    value.as_int().unwrap_or_default();
            }
            "overlay_min_width" => {
                self.overlay_options.min_width = value.as_int().unwrap_or_default();
            }
            "overlay_max_height_percent" => {
                self.overlay_options.max_height_percent = value.as_int().unwrap_or_default();
            }
            "overlay_margin" => {
                self.overlay_options.margin = value.as_int().unwrap_or_default();
            }
            "diagnostic_virtual_text" => {
                self.diagnostic_options.virtual_text = value
                    .as_text()
                    .and_then(super::diagnostic_ui::VirtualTextMode::parse)
                    .unwrap_or_default();
            }
            "diagnostic_virtual_severity" => {
                self.diagnostic_options.virtual_severity = value
                    .as_text()
                    .and_then(super::diagnostic_ui::SeverityFilter::parse)
                    .unwrap_or_default();
            }
            "diagnostic_source" => {
                self.diagnostic_options.source = value
                    .as_text()
                    .and_then(super::diagnostic_ui::SourceDisplay::parse)
                    .unwrap_or_default();
            }
            "diagnostic_counts" => {
                self.diagnostic_options.counts = value
                    .as_text()
                    .and_then(super::diagnostic_ui::CountsMode::parse)
                    .unwrap_or_default();
            }
            "diagnostic_workspace_counts" => {
                self.diagnostic_options.workspace_counts = value.as_bool().unwrap_or(false);
            }
            "explorer_enabled" => {
                self.explorer.set_enabled(value.as_bool().unwrap_or(false));
                self.sync_terminal_panel_size();
            }
            "directory_show_hidden" => {
                self.directory_options.show_hidden = value.as_bool().unwrap_or(false);
            }
            "directory_confirm" => {
                self.directory_options.confirm = value.as_bool().unwrap_or(true);
            }
            "directory_trash" => {
                self.directory_options.trash = value.as_bool().unwrap_or(true);
            }
            "directory_sort" => {
                self.directory_options.sort = value
                    .as_text()
                    .and_then(super::directory::SortMode::parse)
                    .unwrap_or_default();
            }
            "explorer_width" => {
                self.explorer.set_width(
                    value
                        .as_int()
                        .unwrap_or(crate::options::EXPLORER_WIDTH_DEFAULT),
                );
                self.sync_terminal_panel_size();
            }
            "explorer_show_hidden" => self
                .explorer
                .set_show_hidden(value.as_bool().unwrap_or(false)),
            "keyguide" => self.keyguide_enabled = value.as_bool().unwrap_or(true),
            "keyguide_delay" => {
                self.keyguide_delay = std::time::Duration::from_millis(
                    value
                        .as_int()
                        .unwrap_or(crate::options::KEYGUIDE_DELAY_DEFAULT)
                        as u64,
                );
            }
            _ => {}
        }
        if spec.scope == options::OptionScope::Window {
            self.sync_active_window_options();
        }
        Ok(())
    }

    fn apply_completion_key(&mut self, value: &OptionValue, up: bool) {
        let Some(binding) = value
            .as_text()
            .and_then(super::completion::KeyBinding::parse)
        else {
            let name = if up {
                "completion_docs_up"
            } else {
                "completion_docs_down"
            };
            self.show_message(format!(
                "E474: Invalid key binding: {name}={}",
                value.display()
            ));
            return;
        };
        if up {
            self.completion_options.keys.up = binding;
        } else {
            self.completion_options.keys.down = binding;
        }
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

    pub(super) fn option_context(&self) -> OptionContext<'_> {
        OptionContext {
            theme_dir: self.theme_dir.as_deref(),
        }
    }

    pub(super) fn option_value(&self, spec: &OptionSpec) -> OptionValue {
        match spec.name {
            "keymap" => OptionValue::Text(self.flavor.name().to_string()),
            "theme" => OptionValue::Text(self.theme.name.to_string()),
            "history" => OptionValue::Int(self.editor.history_limit()),
            "undo_persist" => OptionValue::Bool(self.undo.enabled()),
            "shiftwidth" => OptionValue::Int(self.editor.shiftwidth()),
            "expandtab" => OptionValue::Bool(self.editor.expandtab()),
            "number" => OptionValue::Bool(self.editor.number()),
            "relativenumber" => OptionValue::Bool(self.editor.relativenumber()),
            "numberwidth" => OptionValue::Int(self.editor.numberwidth()),
            "signcolumn" => OptionValue::Text(self.signcolumn.name().to_string()),
            "git_signs" => OptionValue::Bool(self.git_signs),
            "ignorecase" => OptionValue::Bool(self.editor.ignorecase()),
            "smartcase" => OptionValue::Bool(self.editor.smartcase()),
            "autoindent" => OptionValue::Bool(self.editor.autoindent()),
            "foldmethod" => OptionValue::Text(self.editor.fold_method().name().to_string()),
            "foldcolumn" => OptionValue::Int(self.editor.fold_column()),
            "auto_pairs" => OptionValue::Bool(self.editor.auto_pairs()),
            "match_brackets" => OptionValue::Bool(self.editor.match_brackets()),
            "format_on_save" => OptionValue::Bool(self.format_on_save),
            "autosave" => OptionValue::Text(self.autosave.mode().name().to_string()),
            "autosave_delay" => OptionValue::Int(
                usize::try_from(self.autosave.delay_millis()).unwrap_or(usize::MAX),
            ),
            "auto_completion" => OptionValue::Bool(self.completion_options.auto),
            "signature_help" => OptionValue::Bool(self.completion_options.signature),
            "completion_docs_up" => {
                OptionValue::Text(self.completion_options.keys.up.name().to_string())
            }
            "completion_docs_down" => {
                OptionValue::Text(self.completion_options.keys.down.name().to_string())
            }
            "hover_max_width" => OptionValue::Int(self.overlay_options.hover_max_width),
            "completion_docs_max_width" => {
                OptionValue::Int(self.overlay_options.completion_docs_max_width)
            }
            "diagnostic_popup_max_width" => {
                OptionValue::Int(self.overlay_options.diagnostic_popup_max_width)
            }
            "overlay_min_width" => OptionValue::Int(self.overlay_options.min_width),
            "overlay_max_height_percent" => {
                OptionValue::Int(self.overlay_options.max_height_percent)
            }
            "overlay_margin" => OptionValue::Int(self.overlay_options.margin),
            "diagnostic_virtual_text" => {
                OptionValue::Text(self.diagnostic_options.virtual_text.name().to_string())
            }
            "diagnostic_virtual_severity" => {
                OptionValue::Text(self.diagnostic_options.virtual_severity.name().to_string())
            }
            "diagnostic_source" => {
                OptionValue::Text(self.diagnostic_options.source.name().to_string())
            }
            "diagnostic_counts" => {
                OptionValue::Text(self.diagnostic_options.counts.name().to_string())
            }
            "diagnostic_workspace_counts" => {
                OptionValue::Bool(self.diagnostic_options.workspace_counts)
            }
            "explorer_enabled" => OptionValue::Bool(self.explorer.is_enabled()),
            "explorer_width" => OptionValue::Int(self.explorer.width()),
            "explorer_show_hidden" => OptionValue::Bool(self.explorer.show_hidden()),
            "directory_show_hidden" => OptionValue::Bool(self.directory_options.show_hidden),
            "directory_confirm" => OptionValue::Bool(self.directory_options.confirm),
            "directory_trash" => OptionValue::Bool(self.directory_options.trash),
            "directory_sort" => OptionValue::Text(self.directory_options.sort.name().to_string()),
            "keyguide" => OptionValue::Bool(self.keyguide_enabled),
            "keyguide_delay" => OptionValue::Int(
                usize::try_from(self.keyguide_delay.as_millis()).unwrap_or(usize::MAX),
            ),
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
    fn option_metadata_values_apply_without_errors() {
        let mut app = app();
        let theme_dir = app.theme_dir.clone();
        let context = OptionContext {
            theme_dir: theme_dir.as_deref(),
        };
        for spec in &crate::options::OPTIONS {
            for value in spec.values(&context) {
                let input = options::OptionInput::Text(&value);
                let validated = options::validate_spec(spec, input, &context).unwrap();
                app.message = None;
                app.apply_option(spec, validated).unwrap();
                assert_eq!(app.last_message(), None, "{}={value}", spec.name);
            }
        }
    }

    #[test]
    fn signcolumn_and_git_signs_options_apply_and_query() {
        let mut app = app();
        execute(&mut app, "set signcolumn=always");
        assert_eq!(app.signcolumn, crate::options::SignColumnMode::Always);
        assert_eq!(app.editor.signcolumn_width(), 1);
        execute(&mut app, "set scl?");
        assert_eq!(app.last_message(), Some("signcolumn=always"));
        execute(&mut app, "set scl=auto");
        assert_eq!(app.signcolumn, crate::options::SignColumnMode::Auto);
        assert_eq!(app.editor.signcolumn_width(), 0);
        execute(&mut app, "set git_sign");
        assert!(app.git_signs);
        execute(&mut app, "set git_signs?");
        assert_eq!(app.last_message(), Some("git_signs"));
        execute(&mut app, "set git_signs=false");
        assert!(!app.git_signs);
        execute(&mut app, "set signcolumn=never");
        assert_eq!(app.editor.signcolumn_width(), 0);
    }

    #[test]
    fn keyguide_options_apply_and_query() {
        let mut app = app();
        assert!(app.keyguide_enabled);
        assert_eq!(app.keyguide_delay, std::time::Duration::from_millis(250));
        execute(&mut app, "set keyguide_delay=0");
        assert_eq!(app.keyguide_delay, std::time::Duration::ZERO);
        execute(&mut app, "set kgd?");
        assert_eq!(app.last_message(), Some("keyguide_delay=0"));
        execute(&mut app, "set nokeyguide");
        assert!(!app.keyguide_enabled);
        execute(&mut app, "set keyguide?");
        assert_eq!(app.last_message(), Some("nokeyguide"));
        execute(&mut app, "set keyguide&");
        execute(&mut app, "set keyguide_delay&");
        assert!(app.keyguide_enabled);
        assert_eq!(app.keyguide_delay, std::time::Duration::from_millis(250));
    }

    #[test]
    fn fold_options_apply_and_query() {
        let mut app = app();
        execute(&mut app, "set foldmethod=indent");
        assert_eq!(app.editor.fold_method(), kawi_editor::FoldMethod::Indent);
        execute(&mut app, "set fdm?");
        assert_eq!(app.last_message(), Some("foldmethod=indent"));
        execute(&mut app, "set fdc=0");
        assert_eq!(app.editor.fold_column(), 0);
        execute(&mut app, "set foldcolumn?");
        assert_eq!(app.last_message(), Some("foldcolumn=0"));
        execute(&mut app, "set foldmethod&");
        assert_eq!(app.editor.fold_method(), kawi_editor::FoldMethod::Manual);
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
    fn undo_persistence_can_be_toggled() {
        let mut app = app();
        assert!(app.undo.enabled());
        execute(&mut app, "set noundo_persist");
        assert!(!app.undo.enabled());
        execute(&mut app, "set undo_persist?");
        assert_eq!(app.last_message(), Some("noundo_persist"));
        execute(&mut app, "set undo_persist");
        assert!(app.undo.enabled());
    }

    #[test]
    fn completion_options_are_configurable() {
        let mut app = app();
        assert!(!app.completion_options.auto);
        execute(&mut app, "set auto_completion");
        assert!(app.completion_options.auto);
        execute(&mut app, "set auto_completion?");
        assert_eq!(app.last_message(), Some("auto_completion"));
        execute(&mut app, "set completion_docs_up=ctrl-u");
        assert_eq!(app.completion_options.keys.up.name(), "ctrl-u");
        execute(&mut app, "set completion_docs_up?");
        assert_eq!(app.last_message(), Some("completion_docs_up=ctrl-u"));
        execute(&mut app, "set completion_docs_up=wat");
        assert!(app
            .last_message()
            .is_some_and(|message| message.starts_with("E474: Invalid key binding")));
    }

    #[test]
    fn auto_pairs_can_be_disabled_globally() {
        let mut app = app();
        assert!(app.editor.auto_pairs());
        execute(&mut app, "set noauto_pairs");
        assert!(!app.editor.auto_pairs());
        execute(&mut app, "set auto_pairs?");
        assert_eq!(app.last_message(), Some("noauto_pairs"));
        execute(&mut app, "set auto_pairs");
        assert!(app.editor.auto_pairs());
    }

    #[test]
    fn match_brackets_can_be_disabled_globally() {
        let mut app = app();
        assert!(app.editor.match_brackets());
        execute(&mut app, "set nomatch_brackets");
        assert!(!app.editor.match_brackets());
        execute(&mut app, "set match_brackets?");
        assert_eq!(app.last_message(), Some("nomatch_brackets"));
        execute(&mut app, "set match_brackets");
        assert!(app.editor.match_brackets());
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
