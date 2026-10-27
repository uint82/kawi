use super::super::report::{Check, Level};
use crate::state::StatePaths;
use kawi_lsp::Languages;

pub(crate) fn check(paths: &StatePaths) -> Check {
    let languages = Languages::load(&paths.languages);
    let mut details = Vec::new();
    let mut level = file_level(paths, &languages, &mut details);
    let (missing, enabled) = server_details(&languages, &mut details);
    if missing > 0 {
        level = level.max(Level::Warn);
    }
    if enabled == 0 {
        level = level.max(Level::Info);
    }
    let check = Check::new("languages", "Language servers", level, details);
    if missing > 0 {
        check.with_hint("Install missing servers or disable them in languages.toml.")
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

fn server_details(languages: &Languages, details: &mut Vec<String>) -> (usize, usize) {
    let mut used = 0;
    let mut missing = 0;
    for (name, server) in languages.servers() {
        if !server.enabled {
            details.push(format!("{name}: disabled"));
            continue;
        }
        if !languages.server_is_used(name) {
            details.push(format!("{name}: not used by an enabled language"));
            continue;
        }
        used += 1;
        if server.command.trim().is_empty() {
            details.push(format!("{name}: no command configured"));
            continue;
        }
        match languages.server_path(name) {
            Some(path) => details.push(format!("{name}: {} ({})", server.command, path.display())),
            None => {
                missing += 1;
                details.push(format!("{name}: {} not found in PATH", server.command));
            }
        }
    }
    if used == 0 {
        details.push("no used language servers".to_string());
    }
    (missing, used)
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{executable, paths, TestDir};
    use super::*;
    use std::fs;

    fn write_config(paths: &StatePaths, command: &str) -> String {
        let text = format!(
            "[server.rust-analyzer]\nenabled = false\n\
             [server.pyright]\nenabled = false\n\
             [server.ruff]\nenabled = false\n\
             [server.fake]\ncommand = {command:?}\n\n\
             [[language]]\nname = \"rust\"\nservers = [\"fake\"]\n"
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
    fn ignores_a_missing_server_that_no_language_uses() {
        let dir = TestDir::new("languages_unused");
        let paths = paths(&dir.path);
        let server = dir.path.join("fake-server");
        executable(&server);
        let text = format!(
            "[server.rust-analyzer]\nenabled = false\n\
             [server.pyright]\nenabled = false\n\
             [server.ruff]\nenabled = false\n\
             [server.fake]\ncommand = {:?}\n\
             [server.unused]\ncommand = \"missing-unused-server\"\n\n\
             [[language]]\nname = \"rust\"\nservers = [\"fake\"]\n",
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
