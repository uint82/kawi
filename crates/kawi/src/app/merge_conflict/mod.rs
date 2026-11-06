use std::path::Path;
use std::time::Duration;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use kawi_buffer::TextEdit;
use kawi_editor::Action;
use kawi_git::conflict::{self, ConflictBlock, ConflictChoice};
use kawi_git::OperationState;

use super::editor_sync::EDITOR_SYNC_DEBOUNCE;
use super::git_commit::TaskKind;
use super::App;

mod sync;

const CONTINUE_NAME: &str = "git continue";
const ABORT_NAME: &str = "git abort";
const NO_OPERATION: &str = "No merge, rebase, cherry-pick, or revert in progress";

#[derive(Default)]
pub(super) struct MergeConflictState {
    blocks: Vec<ConflictBlock>,
    buffer_id: Option<String>,
    revision: u64,
    synced: bool,
    pending_abort: Option<OperationState>,
}

impl MergeConflictState {
    pub(super) fn blocks(&self) -> &[ConflictBlock] {
        &self.blocks
    }

    pub(super) fn has_blocks(&self) -> bool {
        !self.blocks.is_empty()
    }

    pub(super) fn has_pending_abort(&self) -> bool {
        self.pending_abort.is_some()
    }

    fn is_current(&self, buffer_id: Option<&str>, revision: u64) -> bool {
        self.synced && self.buffer_id.as_deref() == buffer_id && self.revision == revision
    }

    fn clear(&mut self) -> bool {
        let changed = !self.blocks.is_empty();
        self.blocks.clear();
        self.synced = false;
        changed
    }
}

impl App {
    pub(super) fn sync_merge_conflicts(&mut self) -> bool {
        self.sync_merge_conflicts_with(Duration::ZERO)
    }

    pub(super) fn sync_merge_conflicts_deferred(&mut self) -> bool {
        self.sync_merge_conflicts_with(EDITOR_SYNC_DEBOUNCE)
    }

    fn sync_merge_conflicts_with(&mut self, delay: Duration) -> bool {
        if self.editor.current_buffer_is_readonly() {
            let changed = self.merge_conflict.clear();
            if changed {
                self.update_signcolumn_width();
            }
            return changed;
        }
        let number = self.editor.buffer_number();
        let revision = self.editor.buffer_revision();
        let current = self
            .merge_conflict
            .is_current(self.editor.buffer_id(number), revision);
        if current {
            return false;
        }
        if !delay.is_zero() {
            self.schedule_deferred_editor_sync(delay);
            return false;
        }
        let buffer_id = self.editor.buffer_id(number).map(str::to_string);
        let snapshot = self.editor.document_snapshot();
        let changed = self
            .merge_conflict
            .sync_rope(buffer_id, revision, &snapshot);
        if changed {
            self.update_signcolumn_width();
        }
        changed
    }

    pub(super) fn handle_merge_conflict_navigation(&mut self, forward: bool) -> Result<bool> {
        self.sync_merge_conflicts();
        let line = self.editor.row().saturating_add(1);
        let blocks = self.merge_conflict.blocks();
        let index = if forward {
            conflict::next(blocks, line)
        } else {
            conflict::prev(blocks, line)
        };
        let Some(index) = index else {
            self.show_message("No conflicts".to_string());
            return Ok(false);
        };
        let block = blocks[index].clone();
        let total = blocks.len();
        self.editor.record_jump();
        self.editor.apply(Action::JumpTo(block.start, 0));
        self.editor.apply(Action::MoveToFirstNonBlank);
        self.reveal_cursor();
        self.show_message(format!("Conflict {} of {}", index.saturating_add(1), total));
        Ok(false)
    }

    pub(super) fn handle_merge_conflict_choice(&mut self, choice: ConflictChoice) -> Result<bool> {
        if self.editor.current_buffer_is_readonly() {
            self.show_message("Buffer is read-only".to_string());
            return Ok(false);
        }
        self.sync_merge_conflicts();
        let line = self.editor.row().saturating_add(1);
        let Some(index) = conflict::at(self.merge_conflict.blocks(), line) else {
            self.show_message("No conflict at cursor".to_string());
            return Ok(false);
        };
        let block = self.merge_conflict.blocks()[index].clone();
        let lines = self.editor.text_lines();
        let replacement = conflict::replacement(&lines, &block, choice);
        let edit = replacement_edit(&lines, &block, &replacement);
        let revision = self.editor.buffer_revision();
        if let Err(error) = self.editor.apply_text_edits_at_revision(revision, &[edit]) {
            self.show_message(super::file_error_message(error));
            return Ok(false);
        }
        let row = block.start.min(self.editor.line_count().saturating_sub(1));
        self.editor.apply(Action::JumpTo(row, 0));
        self.reveal_cursor();
        self.sync_merge_conflicts();
        self.show_message(format!("Took {}", choice.label()));
        Ok(false)
    }

    pub(super) fn handle_git_continue(&mut self) -> Result<bool> {
        let Some(status) = self.git_status.status().cloned() else {
            self.show_message("No git repository".to_string());
            return Ok(false);
        };
        let Some(args) = continue_args(status.operation) else {
            self.show_message(NO_OPERATION.to_string());
            return Ok(false);
        };
        if status.has_conflicts() {
            self.show_message("Resolve conflicts before continuing".to_string());
            return Ok(false);
        }
        if let Err(message) = self.reject_staged_conflict_markers(&status.root) {
            self.show_message(message);
            return Ok(false);
        }
        self.start_git_operation_task(args, status.root, TaskKind::Continue, CONTINUE_NAME);
        Ok(false)
    }

    pub(super) fn handle_git_abort(&mut self) -> Result<bool> {
        let Some(operation) = self.git_status.status().map(|status| status.operation) else {
            self.show_message("No git repository".to_string());
            return Ok(false);
        };
        let Some(name) = operation_name(operation) else {
            self.show_message(NO_OPERATION.to_string());
            return Ok(false);
        };
        self.merge_conflict.pending_abort = Some(operation);
        self.show_message(format!("Abort {name}? y/n"));
        Ok(false)
    }

    pub(super) fn handle_git_abort_confirmation(&mut self, key: KeyEvent) -> Result<bool> {
        let Some(operation) = self.merge_conflict.pending_abort else {
            return Ok(false);
        };
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return Ok(false);
        }
        match key.code {
            KeyCode::Char('y' | 'Y') => {
                self.merge_conflict.pending_abort = None;
                let Some(root) = self.git_status.status().map(|status| status.root.clone()) else {
                    return Ok(false);
                };
                if let Some(args) = abort_args(operation) {
                    self.start_git_operation_task(args, root, TaskKind::Abort, ABORT_NAME);
                }
            }
            KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                self.merge_conflict.pending_abort = None;
                self.clear_message();
            }
            _ => {}
        }
        Ok(false)
    }

    pub(super) fn reject_staged_conflict_markers(&self, root: &Path) -> Result<(), String> {
        match kawi_git::index_has_conflict_markers(root) {
            Ok(false) => Ok(()),
            Ok(true) => Err("Staged files still contain conflict markers".to_string()),
            Err(error) => Err(format!("Cannot check staged conflicts: {error}")),
        }
    }

    pub(super) fn reject_worktree_conflict_markers(&self, path: &Path) -> Result<(), String> {
        match kawi_git::file_has_conflict_markers(path) {
            Ok(false) => Ok(()),
            Ok(true) => Err("Resolve conflict markers before staging".to_string()),
            Err(error) => Err(format!(
                "Cannot check conflict markers in {}: {error}",
                path.display()
            )),
        }
    }

    pub(super) fn reject_tracked_conflict_markers(&self, root: &Path) -> Result<(), String> {
        match kawi_git::worktree_has_conflict_markers(root) {
            Ok(false) => Ok(()),
            Ok(true) => Err("Tracked files still contain conflict markers".to_string()),
            Err(error) => Err(format!("Cannot check tracked conflicts: {error}")),
        }
    }
}

fn replacement_edit(lines: &[String], block: &ConflictBlock, replacement: &[&str]) -> TextEdit {
    let start = line_offset(lines, block.start);
    let end = line_offset(lines, block.end.saturating_add(1));
    let mut text = replacement.join("\n");
    if !replacement.is_empty() && block.end.saturating_add(1) < lines.len() {
        text.push('\n');
    }
    TextEdit { start, end, text }
}

fn line_offset(lines: &[String], index: usize) -> usize {
    let mut offset: usize = 0;
    for (position, line) in lines.iter().take(index).enumerate() {
        offset = offset.saturating_add(line.chars().count());
        if position.saturating_add(1) < lines.len() {
            offset = offset.saturating_add(1);
        }
    }
    offset
}

fn continue_args(operation: OperationState) -> Option<&'static [&'static str]> {
    match operation {
        OperationState::Merge => Some(&["merge", "--continue"]),
        OperationState::Rebase => Some(&["rebase", "--continue"]),
        OperationState::CherryPick => Some(&["cherry-pick", "--continue"]),
        OperationState::Revert => Some(&["revert", "--continue"]),
        OperationState::Bisect | OperationState::None => None,
    }
}

fn abort_args(operation: OperationState) -> Option<&'static [&'static str]> {
    match operation {
        OperationState::Merge => Some(&["merge", "--abort"]),
        OperationState::Rebase => Some(&["rebase", "--abort"]),
        OperationState::CherryPick => Some(&["cherry-pick", "--abort"]),
        OperationState::Revert => Some(&["revert", "--abort"]),
        OperationState::Bisect => Some(&["bisect", "reset"]),
        OperationState::None => None,
    }
}

fn operation_name(operation: OperationState) -> Option<&'static str> {
    match operation {
        OperationState::Merge => Some("merge"),
        OperationState::Rebase => Some("rebase"),
        OperationState::CherryPick => Some("cherry-pick"),
        OperationState::Revert => Some("revert"),
        OperationState::Bisect => Some("bisect"),
        OperationState::None => None,
    }
}

#[cfg(test)]
mod tests;
