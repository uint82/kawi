use super::super::report::{Check, Level};
use crate::state::StatePaths;
use std::path::Path;

pub(crate) fn check(paths: &StatePaths, workspace: &Path) -> Check {
    match crate::session::load_for_cwd(&paths.sessions, workspace) {
        Ok(Some(session)) => Check::new(
            "session",
            "Session",
            Level::Ok,
            vec![format!(
                "session: version {}, {} buffers",
                session.version,
                session.buffers.len()
            )],
        ),
        Ok(None) => Check::new(
            "session",
            "Session",
            Level::Info,
            vec!["session: none for this workspace".to_string()],
        ),
        Err(error) => Check::new("session", "Session", Level::Warn, vec![error])
            .with_hint("Remove the damaged session file or let Kawi ignore it."),
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{paths, TestDir};
    use super::*;
    use std::fs;

    #[test]
    fn warns_about_a_corrupt_workspace_session() {
        let dir = TestDir::new("session_corrupt");
        let paths = paths(&dir.path);
        let workspace = dir.path.join("workspace");
        fs::create_dir_all(&workspace).unwrap();
        fs::create_dir_all(&paths.sessions).unwrap();
        let workspace = fs::canonicalize(workspace).unwrap();
        let path = paths
            .sessions
            .join(format!("{}.json", crate::workspace::key(&workspace)));
        fs::write(path, "{not json").unwrap();
        let check = check(&paths, &workspace);
        assert_eq!(check.level, Level::Warn);
        assert!(check.hint.is_some());
    }
}
