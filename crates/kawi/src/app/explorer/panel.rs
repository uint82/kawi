use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use kawi_tui::frame::{ExplorerRow, ExplorerRowKind, Rect};

use super::super::App;
use super::ExplorerState;

pub(crate) struct ExplorerView<'a> {
    pub(crate) rect: Rect,
    pub(crate) title: &'static str,
    pub(crate) status: String,
    pub(crate) focused: bool,
    pub(crate) rows: Vec<ExplorerRow<'a>>,
    pub(crate) selected: Option<usize>,
    pub(crate) offset: usize,
    pub(crate) error: Option<&'a str>,
}

#[derive(Clone, Copy, Default)]
struct BufferMark {
    active: bool,
    dirty: bool,
}

impl App {
    pub(crate) fn sync_explorer_marks(&mut self) {
        if !self.explorer.is_visible() {
            return;
        }
        let Some(root) = self.explorer.root().map(Path::to_path_buf) else {
            return;
        };
        self.explorer.sync_marks(
            &root,
            &self.problems,
            self.git_status.status(),
            self.git_status.revision(),
        );
    }

    pub(crate) fn explorer_view(&self) -> Option<ExplorerView<'_>> {
        let rect = self.explorer_rect()?;
        let buffers = self.explorer_buffer_marks();
        let rows = self
            .explorer
            .rows()
            .iter()
            .map(|row| explorer_row(&self.explorer, row, &buffers))
            .collect();
        Some(ExplorerView {
            rect,
            title: "Explorer",
            status: explorer_status(&self.explorer),
            focused: self.explorer.is_focused(),
            rows,
            selected: self.explorer.selected_index(),
            offset: self.explorer.offset(),
            error: self.explorer.error(),
        })
    }

    fn explorer_buffer_marks(&self) -> BTreeMap<PathBuf, BufferMark> {
        let cwd = std::env::current_dir().unwrap_or_default();
        let active = self.editor.buffer_number();
        self.editor
            .buffer_numbers()
            .into_iter()
            .filter_map(|number| {
                let path = self.editor.buffer_path(number)?;
                let path = kawi_editor::paths::absolute(path, &cwd);
                Some((
                    path,
                    BufferMark {
                        active: number == active,
                        dirty: self.editor.buffer_is_dirty(number).unwrap_or(false),
                    },
                ))
            })
            .collect()
    }
}

fn explorer_row<'a>(
    explorer: &'a ExplorerState,
    row: &'a super::model::ExplorerRow,
    buffers: &BTreeMap<PathBuf, BufferMark>,
) -> ExplorerRow<'a> {
    let directory = matches!(row.kind, ExplorerRowKind::Directory { .. });
    let buffer = buffers.get(&row.path).copied().unwrap_or_default();
    ExplorerRow {
        name: &row.name,
        depth: row.depth,
        kind: row.kind,
        hidden: row.hidden,
        active: buffer.active,
        dirty: buffer.dirty,
        git: explorer.git(&row.path, directory),
        diagnostic: explorer.diagnostic(&row.path, directory),
    }
}

fn explorer_status(explorer: &ExplorerState) -> String {
    let mut status = kawi_tui::text::count_label(explorer.rows().len(), "item", "items");
    if explorer.show_hidden() {
        status.push_str("  hidden");
    }
    if explorer.is_truncated() {
        status.push_str("  truncated");
    }
    status
}
