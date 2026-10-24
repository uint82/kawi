use kawi_buffer::TextEdit;
use kawi_lsp::{DocumentEdit, WorkspaceEdit};

use super::CompletionSession;

impl CompletionSession {
    pub(crate) fn workspace_edit(&self, cursor: usize) -> Option<WorkspaceEdit> {
        let item = self.selected_item()?;
        let primary = adjust_edit(&item.primary, self.event.cursor, cursor);
        let additional = self.additional.get(&item.id).unwrap_or(&item.additional);
        let mut edits = Vec::with_capacity(additional.len().saturating_add(1));
        edits.push(primary);
        edits.extend(
            additional
                .iter()
                .map(|edit| adjust_later_edit(edit, self.event.cursor, cursor)),
        );
        Some(WorkspaceEdit {
            documents: vec![DocumentEdit {
                path: self.event.path.clone(),
                original_hash: self.event.original_hash,
                edits,
            }],
        })
    }
}

fn adjust_edit(edit: &TextEdit, requested: usize, current: usize) -> TextEdit {
    let mut adjusted = edit.clone();
    if requested != current {
        let delta = current as isize - requested as isize;
        adjusted.end = shift(adjusted.end, delta).max(adjusted.start);
    }
    adjusted
}

fn adjust_later_edit(edit: &TextEdit, requested: usize, current: usize) -> TextEdit {
    let mut adjusted = edit.clone();
    if requested != current && adjusted.start >= requested {
        let delta = current as isize - requested as isize;
        adjusted.start = shift(adjusted.start, delta);
        adjusted.end = shift(adjusted.end, delta);
    }
    adjusted
}

fn shift(value: usize, delta: isize) -> usize {
    value.saturating_add_signed(delta)
}
