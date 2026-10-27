use super::super::report::{Check, Level};
use crate::options::OptionValue;
use crate::state::StatePaths;
use kawi_tui::theme::Theme;

const DEFAULT_THEME: &str = "default";

pub(crate) fn check(paths: &StatePaths) -> Check {
    let loaded = crate::config::load(&paths.config);
    let mut details = Vec::new();
    let mut level = config_level(paths, &loaded, &mut details);
    level = level.max(theme_level(paths, &loaded.config, &mut details));
    let check = Check::new("config", "Configuration", level, details);
    if level == Level::Warn {
        check.with_hint("Fix the rejected configuration values listed above.")
    } else {
        check
    }
}

fn config_level(
    paths: &StatePaths,
    loaded: &crate::config::ConfigLoad,
    details: &mut Vec<String>,
) -> Level {
    let mut level = if paths.config.is_file() {
        details.push(format!("config: {}", paths.config.display()));
        Level::Ok
    } else {
        details.push(format!(
            "config: {} (missing, using defaults)",
            paths.config.display()
        ));
        Level::Info
    };
    for warning in &loaded.warnings {
        details.push(warning.format(&paths.config));
        level = level.max(Level::Warn);
    }
    level
}

fn theme_level(
    paths: &StatePaths,
    config: &crate::config::Config,
    details: &mut Vec<String>,
) -> Level {
    let theme = config
        .get("theme")
        .and_then(OptionValue::as_text)
        .unwrap_or(DEFAULT_THEME);
    match Theme::validate(theme, &paths.themes) {
        Ok(()) => {
            details.push(format!("theme: {theme}"));
            Level::Ok
        }
        Err(error) => {
            details.push(format!("theme: {error}"));
            Level::Warn
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{paths, TestDir};
    use super::*;
    use std::fs;

    #[test]
    fn accepts_a_valid_custom_theme() {
        let dir = TestDir::new("config_theme");
        let paths = paths(&dir.path);
        fs::create_dir_all(&paths.themes).unwrap();
        fs::write(
            paths.themes.join("personal.toml"),
            crate::test_support::custom_theme(),
        )
        .unwrap();
        fs::write(&paths.config, "[ui]\ntheme = \"personal\"\n").unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Ok);
        assert!(check
            .details
            .iter()
            .any(|detail| detail == "theme: personal"));
    }

    #[test]
    fn rejects_an_invalid_custom_builtin_theme() {
        let dir = TestDir::new("config_invalid_theme");
        let paths = paths(&dir.path);
        fs::create_dir_all(&paths.themes).unwrap();
        fs::write(paths.themes.join("default.toml"), "invalid").unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Warn);
        assert!(check.hint.is_some());
    }
}
