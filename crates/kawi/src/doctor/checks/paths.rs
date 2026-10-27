use super::super::report::{Check, Level};
use crate::state::StatePaths;
use std::fs;
use std::path::Path;

#[derive(Clone, Copy)]
enum EntryKind {
    File,
    Directory,
}

pub(crate) fn check(paths: Option<&StatePaths>) -> Check {
    let Some(paths) = paths else {
        return unavailable();
    };
    let (details, level) = describe_all(path_entries(paths));
    let check = Check::new("paths", "Paths", level, details);
    if level == Level::Error {
        check.with_hint("Check XDG environment variables and path permissions.")
    } else {
        check
    }
}

fn unavailable() -> Check {
    Check::new(
        "paths",
        "Paths",
        Level::Error,
        vec!["cannot resolve XDG config and state directories".to_string()],
    )
    .with_hint("Set HOME or XDG_CONFIG_HOME and XDG_STATE_HOME.")
}

fn path_entries(paths: &StatePaths) -> Vec<(&'static str, &Path, EntryKind)> {
    let mut entries = vec![
        ("config", paths.config.as_path(), EntryKind::File),
        ("languages", paths.languages.as_path(), EntryKind::File),
        ("tasks", paths.tasks.as_path(), EntryKind::File),
        ("themes", paths.themes.as_path(), EntryKind::Directory),
        ("queries", paths.queries.as_path(), EntryKind::Directory),
    ];
    if let Some(state) = paths.sessions.parent() {
        entries.push(("state", state, EntryKind::Directory));
    }
    entries
}

fn describe_all(entries: Vec<(&'static str, &Path, EntryKind)>) -> (Vec<String>, Level) {
    let mut details = Vec::with_capacity(entries.len());
    let mut level = Level::Ok;
    for (label, path, kind) in entries {
        let (detail, detail_level) = describe(label, path, kind);
        details.push(detail);
        level = level.max(detail_level);
    }
    (details, level)
}

fn describe(label: &str, path: &Path, kind: EntryKind) -> (String, Level) {
    match fs::metadata(path) {
        Ok(metadata) if kind.matches(&metadata) => {
            (format!("{label}: {}", path.display()), Level::Ok)
        }
        Ok(_) => (
            format!("{label}: {} (unexpected file type)", path.display()),
            Level::Warn,
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (
            format!("{label}: {} (missing)", path.display()),
            Level::Info,
        ),
        Err(error) => (
            format!("{label}: {} ({error})", path.display()),
            Level::Error,
        ),
    }
}

impl EntryKind {
    fn matches(self, metadata: &fs::Metadata) -> bool {
        match self {
            Self::File => metadata.is_file(),
            Self::Directory => metadata.is_dir(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{paths, TestDir};
    use super::*;

    #[test]
    fn missing_optional_paths_are_informational() {
        let dir = TestDir::new("paths_missing");
        let paths = paths(&dir.path);
        let check = check(Some(&paths));
        assert_eq!(check.level, Level::Info);
        assert!(check
            .details
            .iter()
            .any(|detail| detail.contains("(missing)")));
    }
}
