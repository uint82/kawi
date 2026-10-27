use super::super::report::{Check, Level};
use super::probe::{self, Probe};
use crate::state::StatePaths;
use std::path::Path;

pub(crate) fn check(paths: &StatePaths) -> Check {
    let mut targets = Vec::with_capacity(4);
    if let Some(state) = paths.sessions.parent() {
        targets.push(("state", state));
    }
    targets.push(("sessions", paths.sessions.as_path()));
    targets.push(("recovery", paths.recovery.as_path()));
    targets.push(("undo", paths.undo.as_path()));
    let mut details = Vec::with_capacity(targets.len());
    let mut level = Level::Ok;
    for (label, path) in targets {
        let (detail, detail_level) = describe(label, path, &probe::probe(path));
        details.push(detail);
        level = level.max(detail_level);
    }
    let check = Check::new("state", "State", level, details);
    if level == Level::Error {
        check.with_hint("Check permissions for the state directories listed above.")
    } else {
        check
    }
}

fn describe(label: &str, path: &Path, probe: &Probe) -> (String, Level) {
    if let Some(error) = probe.error.as_deref() {
        return (
            format!("{label}: {} ({error})", path.display()),
            Level::Error,
        );
    }
    if !probe.writable {
        return (
            format!("{label}: {} (not writable)", path.display()),
            Level::Error,
        );
    }
    if probe.exists {
        (format!("{label}: {} (writable)", path.display()), Level::Ok)
    } else {
        (
            format!(
                "{label}: {} (missing, parent writable: {})",
                path.display(),
                probe.directory.display()
            ),
            Level::Info,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{paths, TestDir};
    use super::*;
    use std::fs;

    #[test]
    fn missing_state_directories_do_not_get_created() {
        let dir = TestDir::new("state_missing");
        let paths = paths(&dir.path);
        let check = check(&paths);
        assert_eq!(check.level, Level::Info);
        assert!(!dir.path.join("state").exists());
    }

    #[test]
    fn a_file_in_place_of_the_state_root_is_an_error() {
        let dir = TestDir::new("state_file");
        let paths = paths(&dir.path);
        fs::write(dir.path.join("state"), "not a directory").unwrap();
        let check = check(&paths);
        assert_eq!(check.level, Level::Error);
        assert!(check.hint.is_some());
    }
}
