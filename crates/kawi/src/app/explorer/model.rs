use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use super::marks::{ExplorerMarks, MarksKey};
use super::tree;
use super::watch::ExplorerWatch;
use super::ExplorerRowKind;
use crate::options::EXPLORER_WIDTH_DEFAULT;

const DOUBLE_CLICK: Duration = Duration::from_millis(400);

#[derive(Clone)]
pub(crate) struct ExplorerRow {
    pub(crate) path: PathBuf,
    pub(crate) name: String,
    pub(crate) depth: usize,
    pub(crate) kind: ExplorerRowKind,
    pub(crate) hidden: bool,
}

pub(crate) struct ExplorerState {
    pub(super) root: Option<PathBuf>,
    pub(super) enabled: bool,
    pub(super) visible: bool,
    pub(super) focused: bool,
    pub(super) width: usize,
    pub(super) show_hidden: bool,
    pub(super) expanded: BTreeSet<PathBuf>,
    pub(super) rows: Vec<ExplorerRow>,
    pub(super) selected: Option<PathBuf>,
    pub(super) offset: usize,
    pub(super) error: Option<String>,
    pub(super) watch_error: Option<String>,
    pub(super) truncated: bool,
    pub(super) marks: ExplorerMarks,
    pub(super) marks_key: MarksKey,
    pub(super) marks_root: Option<PathBuf>,
    pub(super) watch: Option<ExplorerWatch>,
    pub(super) wake: Option<Sender<()>>,
    pub(super) last_click: Option<(Instant, PathBuf)>,
}

impl Default for ExplorerState {
    fn default() -> Self {
        Self {
            root: None,
            enabled: false,
            visible: false,
            focused: false,
            width: EXPLORER_WIDTH_DEFAULT,
            show_hidden: false,
            expanded: BTreeSet::new(),
            rows: Vec::new(),
            selected: None,
            offset: 0,
            error: None,
            watch_error: None,
            truncated: false,
            marks: ExplorerMarks::default(),
            marks_key: MarksKey::default(),
            marks_root: None,
            watch: None,
            wake: None,
            last_click: None,
        }
    }
}

impl ExplorerState {
    pub(crate) fn set_wake(&mut self, wake: Sender<()>) {
        self.wake = Some(wake);
    }

    pub(crate) fn width(&self) -> usize {
        self.width
    }

    pub(crate) fn set_width(&mut self, width: usize) {
        self.width = width;
    }

    pub(crate) fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    pub(crate) fn set_show_hidden(&mut self, show_hidden: bool) {
        if self.show_hidden == show_hidden {
            return;
        }
        self.show_hidden = show_hidden;
        self.refresh_and_watch();
    }

    pub(crate) fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        if !enabled {
            self.close();
        }
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.visible
    }

    pub(crate) fn is_focused(&self) -> bool {
        self.focused
    }

    pub(crate) fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }

    pub(crate) fn open(&mut self, root: PathBuf) {
        if self.root.as_deref() != Some(root.as_path()) {
            self.root = Some(root);
            self.expanded.clear();
            self.selected = None;
            self.marks_root = None;
        }
        self.visible = true;
        self.focused = true;
        self.refresh_and_watch();
    }

    pub(crate) fn close(&mut self) {
        self.visible = false;
        self.focused = false;
        self.watch = None;
        self.watch_error = None;
    }

    pub(crate) fn focus(&mut self) {
        self.visible = true;
        self.focused = true;
        self.sync_watch();
    }

    pub(crate) fn focus_editor(&mut self) {
        self.focused = false;
    }

    pub(crate) fn refresh(&mut self) {
        let Some(root) = self.root.as_ref() else {
            self.rows.clear();
            self.selected = None;
            self.offset = 0;
            return;
        };
        let load = tree::load_rows(root, &self.expanded, self.show_hidden);
        self.rows = load.rows.into_iter().map(ExplorerRow::from).collect();
        self.error = load.error;
        self.truncated = load.truncated;
        self.repair_selection();
        self.offset = self.offset.min(self.rows.len());
    }

    pub(crate) fn rows(&self) -> &[ExplorerRow] {
        &self.rows
    }

    pub(crate) fn offset(&self) -> usize {
        self.offset
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.watch_error.as_deref().or(self.error.as_deref())
    }

    pub(crate) fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub(crate) fn selected_index(&self) -> Option<usize> {
        let selected = self.selected.as_ref()?;
        self.rows.iter().position(|row| row.path == *selected)
    }

    pub(crate) fn selected_row(&self) -> Option<&ExplorerRow> {
        let index = self.selected_index()?;
        self.rows.get(index)
    }

    pub(crate) fn register_click(&mut self, path: &Path) -> bool {
        let now = Instant::now();
        let double = self.last_click.as_ref().is_some_and(|(last, clicked)| {
            clicked == path && now.duration_since(*last) <= DOUBLE_CLICK
        });
        self.last_click = if double {
            None
        } else {
            Some((now, path.to_path_buf()))
        };
        double
    }

    pub(super) fn refresh_and_watch(&mut self) {
        self.refresh();
        self.sync_watch();
    }

    fn repair_selection(&mut self) {
        if self
            .selected
            .as_ref()
            .is_some_and(|selected| self.rows.iter().any(|row| row.path == *selected))
        {
            return;
        }
        let parent = self
            .selected
            .as_ref()
            .and_then(|selected| self.find_parent_row(selected));
        self.selected = parent.or_else(|| self.rows.first().map(|row| row.path.clone()));
        if self.selected.is_none() {
            self.offset = 0;
        }
    }

    fn find_parent_row(&self, path: &Path) -> Option<PathBuf> {
        let root = self.root.as_deref()?;
        let mut current = path.parent();
        while let Some(parent) = current {
            if self.rows.iter().any(|row| row.path == parent) {
                return Some(parent.to_path_buf());
            }
            if parent == root {
                break;
            }
            current = parent.parent();
        }
        None
    }
}

impl From<tree::TreeRow> for ExplorerRow {
    fn from(row: tree::TreeRow) -> Self {
        Self {
            path: row.path,
            name: row.name,
            depth: row.depth,
            kind: row.kind,
            hidden: row.hidden,
        }
    }
}
