use super::super::report::{Check, Level};
use crate::state::StatePaths;
use kawi_lsp::Languages;

struct ServerDetails {
    used: usize,
    missing: usize,
    optional_missing: usize,
}

struct FormatterDetails {
    configured: usize,
    missing: usize,
}

pub(crate) fn check(paths: &StatePaths) -> Check {
    let languages = Languages::load(&paths.languages);
    let mut details = Vec::new();
    let mut level = file_level(paths, &languages, &mut details);
    let servers = server_details(&languages, &mut details);
    let formatters = formatter_details(&languages, &mut details);
    if servers.missing > 0 || formatters.missing > 0 {
        level = level.max(Level::Warn);
    } else if servers.optional_missing > 0 {
        level = level.max(Level::Info);
    }
    if servers.used == 0 && formatters.configured == 0 {
        level = level.max(Level::Info);
    }
    let check = Check::new("languages", "Languages", level, details);
    if servers.missing > 0 || formatters.missing > 0 {
        check.with_hint("Install missing servers or formatters, or disable them in languages.toml.")
    } else {
        check
    }
}

fn file_level(paths: &StatePaths, languages: &Languages, details: &mut Vec<String>) -> Level {
    let mut level = if paths.languages.is_file() {
        details.push(format!("languages: {}", paths.languages.display()));
        Level::Ok
    } else {
        details.push(format!(
            "languages: {} (missing, using defaults)",
            paths.languages.display()
        ));
        Level::Info
    };
    for warning in languages.warnings() {
        details.push(warning.format(&paths.languages));
        level = level.max(Level::Warn);
    }
    level
}

fn server_details(languages: &Languages, details: &mut Vec<String>) -> ServerDetails {
    let mut result = ServerDetails {
        used: 0,
        missing: 0,
        optional_missing: 0,
    };
    for (name, server) in languages.servers() {
        if !server.enabled {
            details.push(format!("{name}: disabled"));
            continue;
        }
        if !languages.server_is_used(name) {
            details.push(format!("{name}: not used by an enabled language"));
            continue;
        }
        result.used += 1;
        if server.command.trim().is_empty() {
            details.push(format!("{name}: no command configured"));
            continue;
        }
        match languages.server_path(name) {
            Some(path) => details.push(format!("{name}: {} ({})", server.command, path.display())),
            None => {
                if languages.server_is_user_configured(name) {
                    result.missing += 1;
                } else {
                    result.optional_missing += 1;
                }
                details.push(format!("{name}: {} not found in PATH", server.command));
            }
        }
    }
    if result.used == 0 {
        details.push("no used language servers".to_string());
    }
    result
}

fn formatter_details(languages: &Languages, details: &mut Vec<String>) -> FormatterDetails {
    let mut result = FormatterDetails {
        configured: 0,
        missing: 0,
    };
    for (language, formatter) in languages.formatters() {
        result.configured += 1;
        if formatter.command.trim().is_empty() {
            result.missing += 1;
            details.push(format!(
                "formatter {}: no command configured",
                language.name
            ));
            continue;
        }
        match kawi_process::find_executable(&formatter.command) {
            Some(path) => details.push(format!(
                "formatter {}: {} ({})",
                language.name,
                formatter.command,
                path.display()
            )),
            None => {
                result.missing += 1;
                details.push(format!(
                    "formatter {}: {} not found in PATH",
                    language.name, formatter.command
                ));
            }
        }
    }
    if result.configured == 0 {
        details.push("no external formatters configured".to_string());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{executable, paths, TestDir};
    use super::*;
    use std::fs;

    fn disabled_defaults(paths: &StatePaths) -> String {
        Languages::load(&paths.languages)
            .servers()
            .map(|(name, _)| format!("[server.{name}]\nenabled = false\n"))
            .collect()
    }

    fn write_config(paths: &StatePaths, command: &str) -> String {
        let text = format!(
            "{}[server.fake]\ncommand = {command:?}\n\n\
             [[language]]\nname = \"rust\"\nservers = [\"fake\"]\n",
            disabled_defaults(paths)
        );
        fs::write(&paths.languages, text).unwrap();
        command.to_string()
    }

    fn write_formatter_config(paths: &StatePaths, command: &str) -> String {
        let text = format!(
            "{}[[language]]\nname = \"rust\"\nservers = []\n\
             formatter = {{ command = {command:?} }}\n",
            disabled_defaults(paths)
        );
        fs::write(&paths.languages, text).unwrap();
        command.to_string()
    }

    #[test]
    fn reports_a_resolved_server_binary() {
        let dir = TestDir::new("languages_ok");
        let paths = paths(&dir.path);
        let server = dir.path.join("fake-server");
        executable(&server);
        let command = write_config(&paths, server.to_str().unwrap());
        let check = check(&paths);
        assert_eq!(check.level, Level::Ok);
        assert!(check
            .details
            .iter()
            .any(|detail| detail.contains(&command) && detail.contains("fake")));
    }

    #[test]
    fn warns_when_a_server_binary_is_missing() {
        let dir = TestDir::new("languages_missing");
        let paths = paths(&dir.path);
        write_config(&paths, "definitely-missing-kawi-server");
        let check = check(&paths);
        assert_eq!(check.level, Level::Warn);
        assert!(check.hint.is_some());
    }

    #[test]
    fn reports_a_resolved_formatter_binary() {
        let dir = TestDir::new("formatter_ok");
        let paths = paths(&dir.path);
        let formatter = dir.path.join("fake-formatter");
        executable(&formatter);
        let command = write_formatter_config(&paths, formatter.to_str().unwrap());
        let check = check(&paths);
        assert_eq!(check.level, Level::Ok);
        assert!(check
            .details
            .iter()
            .any(|detail| detail.contains("formatter rust") && detail.contains(&command)));
    }

    #[test]
    fn warns_when_a_formatter_binary_is_missing() {
        let dir = TestDir::new("formatter_missing");
        let paths = paths(&dir.path);
        write_formatter_config(&paths, "definitely-missing-kawi-formatter");
        let check = check(&paths);
        assert_eq!(check.level, Level::Warn);
        assert!(check.hint.is_some());
    }

    #[test]
    fn ignores_a_missing_server_that_no_language_uses() {
        let dir = TestDir::new("languages_unused");
        let paths = paths(&dir.path);
        let server = dir.path.join("fake-server");
        executable(&server);
        let text = format!(
            "{}[server.fake]\ncommand = {:?}\n\
             [server.unused]\ncommand = \"missing-unused-server\"\n\n\
             [[language]]\nname = \"rust\"\nservers = [\"fake\"]\n",
            disabled_defaults(&paths),
            server.to_str().unwrap()
        );
        fs::write(&paths.languages, text).unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Ok);
        assert!(check
            .details
            .iter()
            .any(|detail| detail.contains("unused: not used")));
    }
}
