use anyhow::Result;
use kawi_buffer::Buffer;

use crate::actions::Action;
use crate::command::CommandLine;
use crate::cursor::Cursor;
use crate::mode::Mode;
use crate::register::Register;
#[cfg(test)]
use crate::register::RegisterKind;
use crate::selection::{SavedVisual, Selection};

mod case_ops;
mod line_ops;
mod motions;
mod operators;
mod search_mode;
mod session;
mod shift_ops;

use session::InsertEdit;

pub struct Editor {
    buffer: Buffer,
    cursor: Cursor,
    mode: Mode,
    command: CommandLine,
    search: CommandLine,
    search_origin: Option<(usize, usize)>,
    search_status: Option<SearchStatus>,
    selection: Option<Selection>,
    previous_visual: Option<SavedVisual>,
    last_insert: Option<(usize, usize)>,
    register: Register,
    count_digits: usize,
    operator_count: Option<usize>,
    insert_repeat: Option<usize>,
    insert_reopen: bool,
    insert_record: Vec<InsertEdit>,
    insert_moved: bool,
}

struct SearchStatus {
    pattern: String,
    index: usize,
    total: usize,
    revision: u64,
    current: (usize, usize),
}

impl Editor {
    pub fn apply(&mut self, action: Action) {
        match action {
            Action::PushCount(digit) => {
                self.push_count_digit(digit);
                return;
            }
            Action::BeginSequence => {
                self.begin_sequence();
                return;
            }
            Action::CancelPending => {
                self.clear_count();
                return;
            }
            _ => {}
        }
        if self.needs_checkpoint(action) {
            self.record_boundary();
        }
        match action {
            Action::MoveUp => self.move_up(),
            Action::MoveDown => self.move_down(),
            Action::MoveLeft => self.move_left(),
            Action::MoveRight => self.move_right(),
            Action::MoveToLineStart => self.move_to_line_start(),
            Action::MoveToLineEnd => self.move_to_line_end(),
            Action::MoveToNextWordStart => self.move_to_next_word_start(),
            Action::MoveToPrevWordStart => self.move_to_prev_word_start(),
            Action::MoveToNextWordEnd => self.move_to_next_word_end(),
            Action::MoveToNextLineStart => self.move_to_next_line_start(),
            Action::MoveToPrevLineStart => self.move_to_prev_line_start(),
            Action::InsertChar(c) => self.insert_char(c),
            Action::InsertNewline => self.insert_newline(),
            Action::DeleteBackward => self.delete_backward(),
            Action::DeleteForward => self.delete_forward(),
            Action::DeleteLine => self.delete_line(),
            Action::OpenLineBelow => self.open_line_below(),
            Action::DeleteWord => self.delete_word(),
            Action::ChangeWord => self.change_word(),
            Action::DeleteWordBackward => self.delete_word_backward(),
            Action::DeleteWordEnd => self.delete_word_end(),
            Action::ChangeWordEnd => self.change_word_end(),
            Action::DeleteInnerWord => self.delete_inner_word(),
            Action::DeleteAroundWord => self.delete_around_word(),
            Action::ChangeInnerWord => self.change_inner_word(),
            Action::ChangeAroundWord => self.change_around_word(),
            Action::DeleteObject { inner, object } => self.delete_object(inner, object),
            Action::ChangeObject { inner, object } => self.change_object(inner, object),
            Action::YankObject { inner, object } => self.yank_object(inner, object),
            Action::FindChar {
                target,
                forward,
                till,
            } => self.find_char_to(target, forward, till, false),
            Action::RepeatFind {
                target,
                forward,
                till,
            } => self.find_char_to(target, forward, till, true),
            Action::DeleteFind {
                target,
                forward,
                till,
            } => self.delete_find(target, forward, till, false),
            Action::DeleteRepeatFind {
                target,
                forward,
                till,
            } => self.delete_find(target, forward, till, true),
            Action::ChangeFind {
                target,
                forward,
                till,
            } => self.change_find(target, forward, till, false),
            Action::ChangeRepeatFind {
                target,
                forward,
                till,
            } => self.change_find(target, forward, till, true),
            Action::YankFind {
                target,
                forward,
                till,
            } => self.yank_find(target, forward, till, false),
            Action::YankRepeatFind {
                target,
                forward,
                till,
            } => self.yank_find(target, forward, till, true),
            Action::YankInnerWord => self.yank_inner_word(),
            Action::YankAroundWord => self.yank_around_word(),
            Action::DeleteSelection => self.delete_selection(),
            Action::YankSelection => self.yank_selection(),
            Action::PasteOverSelection => self.paste_over_selection(),
            Action::ChangeSelection => self.change_selection(),
            Action::DeleteLines => self.delete_touched_lines(),
            Action::YankLines => self.yank_touched_lines(),
            Action::ChangeLines => self.change_touched_lines(),
            Action::ToggleCase => self.toggle_case(),
            Action::Lowercase => self.lowercase(),
            Action::Uppercase => self.uppercase(),
            Action::ToggleCaseAt { motion } => self.toggle_case_at(motion),
            Action::LowercaseAt { motion } => self.lowercase_at(motion),
            Action::UppercaseAt { motion } => self.uppercase_at(motion),
            Action::GotoLastLine => self.goto_last_line(),
            Action::GotoFirstLine => self.goto_first_line(),
            Action::DeleteToLastLine => self.delete_to_last_line(),
            Action::DeleteToFirstLine => self.delete_to_first_line(),
            Action::YankToLastLine => self.yank_to_last_line(),
            Action::YankToFirstLine => self.yank_to_first_line(),
            Action::ChangeToLastLine => self.change_to_last_line(),
            Action::ChangeToFirstLine => self.change_to_first_line(),
            Action::ShiftRight => self.shift_right(),
            Action::ShiftLeft => self.shift_left(),
            Action::Reindent => self.reindent(),
            Action::EnterVisual => self.enter_visual(),
            Action::EnterVisualLine => self.enter_visual_line(),
            Action::ExitVisual => self.exit_visual(),
            Action::CancelVisual => self.cancel_visual(),
            Action::ReselectVisual => self.reselect_visual(),
            Action::SwapSelectionEnds => self.swap_selection_ends(),
            Action::SelectInnerWord => self.select_inner_word(),
            Action::SelectAroundWord => self.select_around_word(),
            Action::SelectObject { inner, object } => self.select_object(inner, object),
            Action::YankLine => self.yank_lines(),
            Action::YankToEnd => self.yank_to_end(),
            Action::DeleteToEnd => self.delete_to_end(),
            Action::DeleteToStart => self.delete_to_start(),
            Action::YankToStart => self.yank_to_start(),
            Action::ChangeToStart => self.change_to_start(),
            Action::ChangeLine => self.change_line(),
            Action::ChangeToEnd => self.change_to_end(),
            Action::PasteAfter => self.paste_after(),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::EnterInsert => {
                self.arm_insert_repeat(false);
                self.enter_insert();
            }
            Action::InsertAtLast => self.insert_at_last(),
            Action::ExitInsert => self.exit_insert(),
            Action::Append => self.append(),
            Action::AppendLineEnd => self.append_line_end(),
            Action::InsertLineStart => self.insert_line_start(),
            Action::InsertBeforeSelection => self.insert_before_selection(),
            Action::InsertAfterSelection => self.insert_after_selection(),
            Action::CollapseSelection => self.collapse_selection(),
            Action::JumpTo(row, col) => self.jump_to(row, col),
            Action::EnterCommand => self.enter_command(),
            Action::PushCommandChar(c) => self.push_command_char(c),
            Action::DeleteCommandChar => self.delete_command_char(),
            Action::CancelCommand => self.cancel_command(),
            Action::ExecuteCommand => self.noop(),
            Action::EnterSearch => self.enter_search(),
            Action::PushSearchChar(c) => self.push_search_char(c),
            Action::DeleteSearchChar => self.delete_search_char(),
            Action::ConfirmSearch => {
                let _ = self.confirm_search();
            }
            Action::CancelSearch => self.cancel_search(),
            Action::NextMatch => self.next_match(),
            Action::PrevMatch => self.prev_match(),
            Action::Save => self.save_silent(),
            Action::Quit | Action::Noop => self.noop(),
            Action::PushCount(_) | Action::BeginSequence | Action::CancelPending => self.noop(),
        }
        self.normalize_cursor();
        self.track_head(action);
        self.clear_count();
    }

    pub fn byte_count(&self) -> usize {
        self.buffer.byte_count()
    }

    pub(super) fn clamp_col(&mut self) {
        let max_col = self.max_col_for_row(self.cursor.row());
        self.cursor.set_col(self.cursor.col().min(max_col));
    }

    pub(super) fn clamp_row(&mut self) {
        let max_row = self.buffer.max_cursor_row();
        self.cursor.set_row(self.cursor.row().min(max_row));
    }

    pub fn col(&self) -> usize {
        self.cursor.col()
    }

    pub fn command_prompt(&self) -> Option<&str> {
        if self.mode == Mode::Command {
            Some(self.command.text())
        } else {
            None
        }
    }

    pub fn command_text(&self) -> &str {
        self.command.text()
    }

    pub fn filename(&self) -> &str {
        self.buffer.path.as_deref().unwrap_or("[No Name]")
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.is_dirty()
    }

    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    pub(super) fn line_width(&self, row: usize) -> usize {
        self.buffer.line_len(row)
    }

    pub(super) fn max_col_for_row(&self, row: usize) -> usize {
        let width = self.line_width(row);
        if self.mode.is_insert() || self.mode.is_visual() {
            width
        } else {
            width.saturating_sub(1)
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    fn needs_checkpoint(&self, action: Action) -> bool {
        self.mode == Mode::Normal && matches!(action, Action::DeleteForward | Action::DeleteLine)
    }

    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer,
            cursor: Cursor::new(),
            mode: Mode::Normal,
            command: CommandLine::new(),
            search: CommandLine::new(),
            search_origin: None,
            search_status: None,
            selection: None,
            previous_visual: None,
            last_insert: None,
            register: Register::new(),
            count_digits: 0,
            operator_count: None,
            insert_repeat: None,
            insert_reopen: false,
            insert_record: Vec::new(),
            insert_moved: false,
        }
    }

    pub(super) fn push_count_digit(&mut self, digit: u8) {
        let digit = usize::from(digit);
        self.count_digits = self.count_digits.saturating_mul(10).saturating_add(digit);
        if self.count_digits > 999_999_999 {
            self.count_digits = 999_999_999;
        }
    }

    pub(super) fn begin_sequence(&mut self) {
        if self.operator_count.is_none() {
            let pending = std::mem::replace(&mut self.count_digits, 0);
            if pending != 0 {
                self.operator_count = Some(pending);
            }
        }
    }

    pub(super) fn has_count(&self) -> bool {
        self.count_digits != 0 || self.operator_count.is_some()
    }

    pub(super) fn take_count(&mut self) -> usize {
        let operator = self.operator_count.take().unwrap_or(1);
        let pending = std::mem::replace(&mut self.count_digits, 0);
        if pending == 0 {
            return operator;
        }
        operator.saturating_mul(pending).min(999_999_999)
    }

    pub(super) fn clear_count(&mut self) {
        self.count_digits = 0;
        self.operator_count = None;
    }

    pub(super) fn record_boundary(&mut self) {
        self.buffer.checkpoint(self.cursor.row(), self.cursor.col());
    }

    pub(super) fn record_boundary_at(&mut self, row: usize, col: usize) {
        self.buffer.checkpoint(row, col);
    }

    pub(super) fn record_maybe(&mut self) {
        self.buffer
            .maybe_checkpoint(self.cursor.row(), self.cursor.col());
    }

    #[cfg(test)]
    pub fn register_kind(&self) -> RegisterKind {
        self.register.kind()
    }

    #[cfg(test)]
    pub fn register_text(&self) -> String {
        self.register.text().to_string()
    }

    pub fn row(&self) -> usize {
        self.cursor.row()
    }

    pub fn save(&mut self) -> Result<()> {
        self.buffer.save()
    }

    pub fn search_pattern(&self) -> &str {
        self.search.text()
    }

    pub fn search_prompt(&self) -> Option<&str> {
        if self.mode == Mode::Search {
            Some(self.search.text())
        } else {
            None
        }
    }

    pub fn search_status_text(&self) -> Option<String> {
        let status = self.search_status.as_ref()?;
        if status.revision != self.buffer.revision() {
            return None;
        }
        Some(search_mode::status_line(
            self.search.text(),
            status.index,
            status.total,
        ))
    }

    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        let selection = self.selection.as_ref()?;
        if self.mode != Mode::VisualLine {
            return Some((selection.start(), selection.end()));
        }
        let ((start_row, _), (end_row, _)) = (selection.start(), selection.end());
        let end_col = self.buffer.line_len(end_row);
        Some(((start_row, 0), (end_row, end_col)))
    }

    #[cfg(test)]
    pub fn text_lines(&self) -> Vec<String> {
        self.buffer
            .window_lines(0, self.buffer.line_count().saturating_add(1))
    }

    fn track_head(&mut self, action: Action) {
        if !is_tracking(action) {
            return;
        }
        self.sync_head();
    }

    pub fn visible_lines(&self, top: usize, height: usize) -> Vec<String> {
        self.buffer.window_lines(top, height)
    }
}

fn is_tracking(action: Action) -> bool {
    matches!(
        action,
        Action::MoveUp
            | Action::MoveDown
            | Action::MoveLeft
            | Action::MoveRight
            | Action::MoveToLineStart
            | Action::MoveToLineEnd
            | Action::MoveToNextWordStart
            | Action::MoveToPrevWordStart
            | Action::MoveToNextWordEnd
            | Action::FindChar { .. }
            | Action::RepeatFind { .. }
            | Action::GotoLastLine
            | Action::GotoFirstLine
            | Action::JumpTo(_, _)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Mode;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn starts_in_normal() {
        let e = editor_with("hi");
        assert_eq!(e.mode(), Mode::Normal);
    }
}
