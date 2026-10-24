use kawi_editor::{BufferOptionDefaults, WindowOptions};

use super::App;
use crate::keymap::Flavor;
use crate::options::OptionValue;
use crate::state::{self, StatePaths};

impl App {
    pub fn load_state(&mut self, cli_flavor: Option<Flavor>) {
        let Some(paths) = StatePaths::resolve() else {
            return;
        };
        let loaded = crate::config::load(&paths.config);
        let _ =
            crate::config::write_warning_log(&paths.config_log, &paths.config, &loaded.warnings);
        if !loaded.warnings.is_empty() {
            self.show_startup_warning(config_warning_message(
                loaded.warnings.len(),
                &paths.config_log,
            ));
        }
        self.apply_config(&loaded.config);
        if let Some(flavor) = cli_flavor {
            let _ = self.apply_cli_flavor(flavor);
        }
        state::load(&mut self.editor, &paths);
        if let Some(warning) = self.lsp.load(&paths.languages, self.events.wake()) {
            self.show_startup_warning(warning);
        }
    }

    fn apply_config(&mut self, config: &crate::config::Config) {
        for (name, value) in config.values() {
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
                .unwrap_or(true),
        };
        self.editor.set_buffer_defaults(defaults);
        let window = WindowOptions {
            number: config
                .get("number")
                .and_then(OptionValue::as_bool)
                .unwrap_or(true),
            relativenumber: config
                .get("relativenumber")
                .and_then(OptionValue::as_bool)
                .unwrap_or(false),
            numberwidth: config
                .get("numberwidth")
                .and_then(OptionValue::as_int)
                .unwrap_or(4),
        };
        self.apply_window_options(window);
    }
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
    fn warning_summary_mentions_the_log() {
        assert_eq!(
            config_warning_message(3, std::path::Path::new("state/kawi/config.log")),
            "3 config warnings, see state/kawi/config.log"
        );
    }
}
