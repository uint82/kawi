use kawi_editor::{BufferOptionDefaults, WindowOptions};
use kawi_lsp::LanguageWarning;

use super::App;
use crate::keymap::{Flavor, Keymap};
use crate::options::OptionValue;
use crate::state::{self, StatePaths};

impl App {
    pub fn load_state(&mut self, cli_flavor: Option<Flavor>, load_plugins: bool) {
        let Some(paths) = StatePaths::resolve() else {
            return;
        };
        self.theme_dir = Some(paths.themes.clone());
        kawi_syntax::set_user_query_dir(paths.queries.clone());
        let loaded = crate::config::load(&paths.config);
        let keymap_load = crate::keymap::config::load(&paths.keymaps, &self.commands);
        self.keymaps_path = Some(paths.keymaps.clone());
        let language_warnings = self.lsp.load(&paths.languages, self.events.wake());
        let task_load = kawi_task::load(&paths.tasks);
        let mut tasks = task_load.tasks;
        let mut warnings = startup_warning_lines(
            (&loaded.warnings, &paths.config),
            (&keymap_load.warnings, &paths.keymaps),
            (&language_warnings, &paths.languages),
            (&task_load.warnings, &paths.tasks),
        );
        let project_tasks = self.project_tasks_path();
        if project_tasks != paths.tasks {
            let project_load = kawi_task::load(&project_tasks);
            warnings.extend(
                project_load
                    .warnings
                    .iter()
                    .map(|warning| warning.format(&project_tasks)),
            );
            tasks.merge(project_load.tasks);
        }
        self.set_tasks(tasks);
        self.undo.configure(paths.undo.clone());
        self.apply_keymaps(keymap_load);
        self.apply_config(&loaded.config);
        let workspace = self.project_root();
        self.plugin_manager.configure(&paths);
        if let Some(warning) = self.plugin_manager.warning() {
            warnings.push(warning.to_string());
        }
        let plugin_warnings = self.plugins.load(
            crate::app::plugin::PluginLoadContext {
                installed: &paths.installed_plugins,
                dev: &paths.plugins,
                lock: self.plugin_manager.lock(),
                workspace: &workspace,
                settings: &paths.plugin_settings,
                commands: &mut self.commands,
                keymaps: &mut self.keymaps,
                leaders: &self.leaders,
                wake: self.events.wake(),
            },
            load_plugins,
        );
        warnings.extend(plugin_warnings);
        self.rebuild_plugin_styles();
        let _ = crate::config::write_warning_log(&paths.config_log, &warnings);
        if !warnings.is_empty() {
            self.show_startup_warning(config_warning_message(warnings.len(), &paths.config_log));
        }
        self.refresh_command_completion();
        self.undo.spawn_prune();
        if let Some(flavor) = cli_flavor {
            let _ = self.apply_cli_flavor(flavor);
        }
        state::load(&mut self.editor, &paths);
        self.sync_language_settings();
        self.sync_undo_histories();
    }

    pub(super) fn apply_keymaps(&mut self, load: crate::keymap::config::KeymapLoad) {
        self.leaders = load.leader.clone();
        self.keymap.set_leaders(load.leader);
        self.keymaps.set_timeout(load.timeout);
        self.keymaps.set_bindings(load.bindings);
        self.plugins.apply_bindings(&mut self.keymaps);
    }

    fn project_tasks_path(&self) -> std::path::PathBuf {
        self.project_root().join(kawi_task::PROJECT_TASKS)
    }

    fn apply_config(&mut self, config: &crate::config::Config) {
        self.statusline = config.statusline().clone();
        for (name, value) in config.values() {
            if matches!(*name, "shiftwidth" | "expandtab") {
                continue;
            }
            let Some(spec) = crate::options::lookup(name) else {
                continue;
            };
            let _ = self.apply_option(spec, value.clone());
        }
        if let Some(exclusions) = config.undo_exclude() {
            self.undo.set_exclusions(exclusions.to_vec());
        }
        let defaults = BufferOptionDefaults {
            shiftwidth: config.get("shiftwidth").and_then(OptionValue::as_int),
            expandtab: config.get("expandtab").and_then(OptionValue::as_bool),
            autoindent: config
                .get("autoindent")
                .and_then(OptionValue::as_bool)
                .unwrap_or_else(|| default_bool("autoindent")),
        };
        self.editor.set_buffer_defaults(defaults);
        let window = WindowOptions {
            number: config
                .get("number")
                .and_then(OptionValue::as_bool)
                .unwrap_or_else(|| default_bool("number")),
            relativenumber: config
                .get("relativenumber")
                .and_then(OptionValue::as_bool)
                .unwrap_or_else(|| default_bool("relativenumber")),
            numberwidth: config
                .get("numberwidth")
                .and_then(OptionValue::as_int)
                .unwrap_or_else(|| default_int("numberwidth")),
        };
        self.apply_window_options(window);
    }
}

fn default_bool(name: &str) -> bool {
    crate::options::lookup(name)
        .and_then(|spec| spec.default_value().as_bool())
        .unwrap_or(false)
}

fn default_int(name: &str) -> usize {
    crate::options::lookup(name)
        .and_then(|spec| spec.default_value().as_int())
        .unwrap_or_default()
}

fn startup_warning_lines(
    config: (&[crate::config::ConfigWarning], &std::path::Path),
    keymaps: (&[crate::config::ConfigWarning], &std::path::Path),
    languages: (&[LanguageWarning], &std::path::Path),
    tasks: (&[kawi_task::TaskWarning], &std::path::Path),
) -> Vec<String> {
    let mut lines = config
        .0
        .iter()
        .map(|warning| warning.format(config.1))
        .collect::<Vec<_>>();
    lines.extend(keymaps.0.iter().map(|warning| warning.format(keymaps.1)));
    lines.extend(
        languages
            .0
            .iter()
            .map(|warning| warning.format(languages.1)),
    );
    lines.extend(tasks.0.iter().map(|warning| warning.format(tasks.1)));
    lines
}

fn config_warning_message(count: usize, path: &std::path::Path) -> String {
    let noun = if count == 1 {
        "config warning"
    } else {
        "config warnings"
    };
    format!("{count} {noun}, see {}", path.display())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config;
    use kawi_buffer::Buffer;
    use std::fs;
    use std::path::PathBuf;

    fn app() -> App {
        App::new(Buffer::from_string("alpha"), Flavor::Vim)
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_app_config_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn config_applies_runtime_and_future_buffer_defaults() {
        let dir = temp_dir("runtime");
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r#"
[ui]
keymap = "helix"
theme = "gruvbox-dark"
hover_max_width = 80
completion_docs_max_width = 50
overlay_min_width = 24
overlay_max_height_percent = 60
overlay_margin = 4
diagnostic_popup_max_width = 90
diagnostic_virtual_text = "all"
diagnostic_virtual_severity = "information"
diagnostic_source = "always"
diagnostic_counts = "all"
diagnostic_workspace_counts = true

[editor]
shiftwidth = 2
expandtab = true
autoindent = false
auto_pairs = false
match_brackets = false
number = false
ignorecase = true
format_on_save = true
autosave = "idle"
autosave_delay = 250
signcolumn = "always"
git_signs = true
keyguide = false
keyguide_delay = 0

[history]
limit = 25
undo_persist = false
undo_exclude = ["*.gpg"]
"#,
        )
        .unwrap();
        let loaded = config::load(&path);
        let mut app = app();
        app.apply_config(&loaded.config);
        assert_eq!(app.flavor, Flavor::Helix);
        assert_eq!(app.theme.name, "gruvbox-dark");
        assert_eq!(app.overlay_options.hover_max_width, 80);
        assert_eq!(app.overlay_options.completion_docs_max_width, 50);
        assert_eq!(app.overlay_options.min_width, 24);
        assert_eq!(app.overlay_options.max_height_percent, 60);
        assert_eq!(app.overlay_options.margin, 4);
        assert_eq!(app.overlay_options.diagnostic_popup_max_width, 90);
        assert_eq!(
            app.diagnostic_options.virtual_text,
            super::super::diagnostic_ui::VirtualTextMode::All
        );
        assert_eq!(
            app.diagnostic_options.virtual_severity,
            super::super::diagnostic_ui::SeverityFilter::Information
        );
        assert_eq!(
            app.diagnostic_options.source,
            super::super::diagnostic_ui::SourceDisplay::Always
        );
        assert_eq!(
            app.diagnostic_options.counts,
            super::super::diagnostic_ui::CountsMode::All
        );
        assert!(app.diagnostic_options.workspace_counts);
        assert_eq!(app.editor.shiftwidth(), 2);
        assert!(app.editor.expandtab());
        assert!(!app.editor.autoindent());
        assert!(!app.editor.auto_pairs());
        assert!(!app.editor.match_brackets());
        assert!(!app.editor.number());
        assert!(app.editor.ignorecase());
        assert!(app.format_on_save);
        assert_eq!(app.autosave.mode(), crate::options::AutosaveMode::Idle);
        assert_eq!(app.autosave.delay_millis(), 250);
        assert_eq!(app.signcolumn, crate::options::SignColumnMode::Always);
        assert!(app.git_signs);
        assert!(!app.keyguide_enabled);
        assert_eq!(app.keyguide_delay, std::time::Duration::ZERO);
        assert_eq!(app.editor.signcolumn_width(), 1);
        assert_eq!(app.editor.history_limit(), 25);
        assert!(!app.undo.enabled());
        assert_eq!(app.undo.exclusions(), ["*.gpg"]);
        app.editor.open_empty_buffer();
        assert_eq!(app.editor.shiftwidth(), 2);
        assert!(app.editor.expandtab());
        assert!(!app.editor.autoindent());
        assert!(!app.editor.match_brackets());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn config_applies_custom_themes() {
        let dir = temp_dir("custom_theme");
        let themes = dir.join("themes");
        fs::create_dir_all(&themes).unwrap();
        fs::write(
            themes.join("personal.toml"),
            crate::test_support::custom_theme(),
        )
        .unwrap();
        let path = dir.join("config.toml");
        fs::write(&path, "[ui]\ntheme = \"personal\"\n").unwrap();
        let loaded = config::load(&path);
        let mut app = app();
        app.theme_dir = Some(themes);
        app.apply_config(&loaded.config);
        assert_eq!(app.theme_name(), "personal");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn cli_keymap_overrides_config() {
        let dir = temp_dir("cli");
        let path = dir.join("config.toml");
        fs::write(&path, "[ui]\nkeymap = \"helix\"\n").unwrap();
        let loaded = config::load(&path);
        let mut app = app();
        app.apply_config(&loaded.config);
        app.apply_cli_flavor(Flavor::Vim).unwrap();
        assert_eq!(app.flavor, Flavor::Vim);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn startup_warnings_include_every_configuration_source() {
        let dir = temp_dir("warning_lines");
        let config_path = dir.join("config.toml");
        let keymaps_path = dir.join("keymaps.toml");
        let languages_path = dir.join("languages.toml");
        let tasks_path = dir.join("tasks.toml");
        fs::write(&config_path, "[editor]\nshiftwidth = \"wide\"\n").unwrap();
        fs::write(&keymaps_path, "[normal]\n\"gx\" = \"missing.command\"\n").unwrap();
        fs::write(&tasks_path, "[task.bad]\nargs = [\"x\"]\n").unwrap();
        let loaded = config::load(&config_path);
        let keymap_load =
            crate::keymap::config::load(&keymaps_path, &crate::command::CommandRegistry::new());
        let language_warnings =
            vec![
                LanguageWarning::new("language rust: expandtab must be a boolean").at_line(Some(3)),
            ];
        let task_load = kawi_task::load(&tasks_path);
        let lines = startup_warning_lines(
            (&loaded.warnings, &config_path),
            (&keymap_load.warnings, &keymaps_path),
            (&language_warnings, &languages_path),
            (&task_load.warnings, &tasks_path),
        );
        assert_eq!(lines.len(), 4);
        assert!(lines[0].starts_with(&config_path.display().to_string()));
        assert!(lines[1].contains("unknown command"));
        assert!(lines[2].starts_with(&format!("{}:3:", languages_path.display())));
        assert!(lines[3].contains("rejected missing command"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn explorer_config_section_applies_directory_options() {
        let dir = temp_dir("explorer_section");
        let path = dir.join("config.toml");
        fs::write(
            &path,
            r#"
[explorer]
show_hidden = true
confirm = false
trash = false
sort = "size"
sort_reverse = true
sort_directories_first = false
metadata = ["permissions", "size"]
"#,
        )
        .unwrap();
        let loaded = config::load(&path);
        assert!(loaded.warnings.is_empty());
        let mut app = app();
        app.apply_config(&loaded.config);
        assert!(app.directory_options.show_hidden);
        assert!(!app.directory_options.confirm);
        assert!(!app.directory_options.trash);
        assert_eq!(
            app.directory_options.sort.key,
            crate::app::directory::SortKey::Size
        );
        assert!(app.directory_options.sort.reverse);
        assert!(!app.directory_options.sort.directories_first);
        assert_eq!(app.directory_options.metadata.name(), "permissions,size");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn warning_summary_mentions_the_log() {
        assert_eq!(
            config_warning_message(3, std::path::Path::new("state/kawi/config.log")),
            "3 config warnings, see state/kawi/config.log"
        );
    }
}
