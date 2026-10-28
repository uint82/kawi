use super::super::report::{Check, Level};
use crate::command::CommandRegistry;
use crate::state::StatePaths;

pub(crate) fn check(paths: &StatePaths) -> Check {
    let loaded = crate::keymap::config::load(&paths.keymaps, &CommandRegistry::new());
    let mut details = Vec::new();
    let mut level = if paths.keymaps.is_file() {
        details.push(format!("keymaps: {}", paths.keymaps.display()));
        Level::Ok
    } else {
        details.push(format!(
            "keymaps: {} (missing, using defaults)",
            paths.keymaps.display()
        ));
        Level::Info
    };
    for warning in &loaded.warnings {
        details.push(warning.format(&paths.keymaps));
        level = level.max(Level::Warn);
    }
    let check = Check::new("keymaps", "Keymaps", level, details);
    if level == Level::Warn {
        check.with_hint("Fix the rejected keymap entries listed above.")
    } else {
        check
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{paths, TestDir};
    use super::*;
    use std::fs;

    #[test]
    fn accepts_a_valid_keymap_file() {
        let dir = TestDir::new("keymaps_valid");
        let paths = paths(&dir.path);
        fs::write(&paths.keymaps, "[normal]\n\"gx\" = \"file.save\"\n").unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Ok);
    }

    #[test]
    fn reports_unknown_commands() {
        let dir = TestDir::new("keymaps_invalid");
        let paths = paths(&dir.path);
        fs::write(&paths.keymaps, "[normal]\n\"gx\" = \"missing.command\"\n").unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Warn);
        assert!(check.hint.is_some());
    }

    #[test]
    fn missing_file_is_informational() {
        let dir = TestDir::new("keymaps_missing");
        let paths = paths(&dir.path);
        let check = check(&paths);
        assert_eq!(check.level, Level::Info);
    }
}
