use kawi_editor::{BufferOptionDefaults, WindowOptions};
use kawi_lsp::LanguageWarning;

use super::App;
use crate::keymap::Flavor;
use crate::options::OptionValue;
use crate::state::{self, StatePaths};

impl App {
    pub fn load_state(&mut self, cli_flavor: Option<Flavor>) {
        let Some(paths) = StatePaths::resolve() else {
            return;
        };
        self.theme_dir = Some(paths.themes.clone());
        let loaded = crate::config::load(&paths.config);
        let language_warnings = self.lsp.load(&paths.languages, self.events.wake());
        let task_load = kawi_task::load(&paths.tasks);
        let mut tasks = task_load.tasks;
        let mut warnings = startup_warning_lines(
            &loaded.warnings,
            &paths.config,
            &language_warnings,
            &paths.languages,
            &task_load.warnings,
            &paths.tasks,
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
        let _ = crate::config::write_warning_log(&paths.config_log, &warnings);
        if !warnings.is_empty() {
            self.show_startup_warning(config_warning_message(warnings.len(), &paths.config_log));
        }
        self.set_tasks(tasks);
        self.apply_config(&loaded.config);
        if let Some(flavor) = cli_flavor {
            let _ = self.apply_cli_flavor(flavor);
        }
        state::load(&mut self.editor, &paths);
        self.sync_language_indent();
    }

    fn project_tasks_path(&self) -> std::path::PathBuf {
        self.picker_root().join(super::task::PROJECT_TASKS)
    }

    fn apply_config(&mut self, config: &crate::config::Config) {
        for (name, value) in config.values() {
            if matches!(*name, "shiftwidth" | "expandtab") {
                continue;
            }
            let Some(spec) = crate::options::lookup(name) else {
                continue;
            };
            let _ = self.apply_option(spec, value.clone());
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
    config_warnings: &[crate::config::ConfigWarning],
    config_path: &std::path::Path,
    language_warnings: &[LanguageWarning],
    language_path: &std::path::Path,
    task_warnings: &[kawi_task::TaskWarning],
    task_path: &std::path::Path,
) -> Vec<String> {
    let mut lines = config_warnings
        .iter()
        .map(|warning| warning.format(config_path))
        .collect::<Vec<_>>();
    lines.extend(
        language_warnings
            .iter()
            .map(|warning| warning.format(language_path)),
    );
    lines.extend(
        task_warnings
            .iter()
            .map(|warning| warning.format(task_path)),
    );
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

[editor]
shiftwidth = 2
expandtab = true
autoindent = false
number = false
ignorecase = true

[history]
limit = 25
"#,
        )
        .unwrap();
        let loaded = config::load(&path);
        let mut app = app();
        app.apply_config(&loaded.config);
        assert_eq!(app.flavor, Flavor::Helix);
        assert_eq!(app.theme.name, "gruvbox-dark");
        assert_eq!(app.editor.shiftwidth(), 2);
        assert!(app.editor.expandtab());
        assert!(!app.editor.autoindent());
        assert!(!app.editor.number());
        assert!(app.editor.ignorecase());
        assert_eq!(app.editor.history_limit(), 25);
        app.editor.open_empty_buffer();
        assert_eq!(app.editor.shiftwidth(), 2);
        assert!(app.editor.expandtab());
        assert!(!app.editor.autoindent());
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
        let languages_path = dir.join("languages.toml");
        let tasks_path = dir.join("tasks.toml");
        fs::write(&config_path, "[editor]\nshiftwidth = \"wide\"\n").unwrap();
        fs::write(&tasks_path, "[task.bad]\nargs = [\"x\"]\n").unwrap();
        let loaded = config::load(&config_path);
        let language_warnings =
            vec![
                LanguageWarning::new("language rust: expandtab must be a boolean").at_line(Some(3)),
            ];
        let task_load = kawi_task::load(&tasks_path);
        let lines = startup_warning_lines(
            &loaded.warnings,
            &config_path,
            &language_warnings,
            &languages_path,
            &task_load.warnings,
            &tasks_path,
        );
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with(&config_path.display().to_string()));
        assert!(lines[1].starts_with(&format!("{}:3:", languages_path.display())));
        assert!(lines[2].contains("rejected missing command"));
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
