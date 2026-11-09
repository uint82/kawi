use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

use crate::options::{EXPLORER_WIDTH_MAX, EXPLORER_WIDTH_MIN};
use crate::session::SessionExplorer;

use super::ExplorerState;

impl ExplorerState {
    pub(crate) fn snapshot(&self) -> SessionExplorer {
        let root = self.root.as_deref();
        SessionExplorer {
            visible: self.visible,
            width: self.width,
            root: root.map(|root| root.to_string_lossy().into_owned()),
            show_hidden: self.show_hidden,
            expanded: root
                .map(|root| relative_paths(root, &self.expanded))
                .unwrap_or_default(),
            selected: root
                .zip(self.selected.as_deref())
                .and_then(|(root, path)| relative_path(root, path)),
        }
    }

    pub(crate) fn restore(&mut self, cwd: &Path, session: Option<&SessionExplorer>) {
        if !self.enabled {
            return;
        }
        let Some(session) = session else {
            return;
        };
        let root = session
            .root
            .as_deref()
            .map(PathBuf::from)
            .filter(|path| path.is_absolute() && path.is_dir())
            .unwrap_or_else(|| kawi_picker::project_root(cwd));
        self.root = Some(root.clone());
        self.visible = session.visible;
        self.focused = false;
        self.width = session.width.clamp(EXPLORER_WIDTH_MIN, EXPLORER_WIDTH_MAX);
        self.show_hidden = session.show_hidden;
        self.expanded = session
            .expanded
            .iter()
            .filter_map(|path| safe_join(&root, path))
            .filter(|path| path.is_dir())
            .collect();
        self.selected = session
            .selected
            .as_deref()
            .and_then(|path| safe_join(&root, path));
        self.offset = 0;
        self.error = None;
        self.watch_error = None;
        self.truncated = false;
        self.marks_root = None;
        self.last_click = None;
        self.refresh();
        self.sync_watch();
    }
}

fn relative_paths(root: &Path, paths: &BTreeSet<PathBuf>) -> Vec<String> {
    paths
        .iter()
        .filter_map(|path| relative_path(root, path))
        .collect()
}

fn relative_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    (!relative.as_os_str().is_empty()).then(|| relative.to_string_lossy().into_owned())
}

fn safe_join(root: &Path, text: &str) -> Option<PathBuf> {
    let path = Path::new(text);
    if path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return None;
    }
    Some(root.join(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::temp_dir;

    fn enabled_state() -> ExplorerState {
        let mut state = ExplorerState::default();
        state.set_enabled(true);
        state
    }

    #[test]
    fn snapshot_and_restore_round_trip_tree_state() {
        let dir = temp_dir("explorer_session");
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/main.rs"), "").unwrap();
        std::fs::write(dir.join("README.md"), "").unwrap();
        let mut state = enabled_state();
        state.open(dir.clone());
        state.expand_directory(&dir.join("src"));
        state.select(dir.join("src/main.rs"));
        let snapshot = state.snapshot();
        let mut restored = enabled_state();
        restored.restore(&dir, Some(&snapshot));
        assert!(restored.is_visible());
        assert_eq!(restored.width(), state.width());
        assert_eq!(
            restored.selected_row().map(|row| row.path.as_path()),
            Some(dir.join("src/main.rs").as_path())
        );
        assert_eq!(restored.rows().len(), 3);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn disabled_explorer_ignores_open_and_restore() {
        let dir = temp_dir("explorer_disabled");
        std::fs::write(dir.join("a.txt"), "").unwrap();
        let session = SessionExplorer {
            visible: true,
            width: 30,
            root: Some(dir.to_string_lossy().into_owned()),
            show_hidden: false,
            expanded: Vec::new(),
            selected: None,
        };
        let mut state = ExplorerState::default();
        state.open(dir.clone());
        state.restore(&dir, Some(&session));
        assert!(!state.is_visible());
        assert!(state.rows().is_empty());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn missing_session_root_falls_back_to_project_root() {
        let dir = temp_dir("explorer_root_fallback");
        std::fs::write(dir.join("a.txt"), "").unwrap();
        let session = SessionExplorer {
            visible: true,
            width: 30,
            root: Some(dir.join("missing").to_string_lossy().into_owned()),
            show_hidden: false,
            expanded: Vec::new(),
            selected: None,
        };
        let mut state = enabled_state();
        state.restore(&dir, Some(&session));
        let canonical = std::fs::canonicalize(&dir).unwrap();
        assert_eq!(state.root(), Some(canonical.as_path()));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unsafe_session_paths_are_ignored() {
        let dir = temp_dir("explorer_unsafe");
        std::fs::write(dir.join("a.txt"), "").unwrap();
        let session = SessionExplorer {
            visible: true,
            width: 30,
            root: None,
            show_hidden: false,
            expanded: vec!["../outside".to_string()],
            selected: Some("/tmp/outside".to_string()),
        };
        let mut state = enabled_state();
        state.restore(&dir, Some(&session));
        let canonical = std::fs::canonicalize(&dir).unwrap();
        assert!(state
            .rows()
            .iter()
            .all(|row| row.path.starts_with(&canonical)));
        assert!(state.selected_row().is_some());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn restored_width_is_clamped() {
        let dir = temp_dir("explorer_width");
        std::fs::write(dir.join("a.txt"), "").unwrap();
        let session = SessionExplorer {
            visible: true,
            width: usize::MAX,
            root: None,
            show_hidden: false,
            expanded: Vec::new(),
            selected: None,
        };
        let mut state = enabled_state();
        state.restore(&dir, Some(&session));
        assert_eq!(state.width(), EXPLORER_WIDTH_MAX);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
