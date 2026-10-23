use kawi_buffer::Buffer;
use ropey::Rope;

use crate::actions::{Action, CaseMotion, ShiftMotion};
use crate::command::CommandLine;
use crate::cursor::Cursor;
use crate::jumps::Jumps;
use crate::marks::Marks;
use crate::mode::Mode;
use crate::register::RegisterKind;
use crate::selection::{SavedVisual, Selection};
use crate::viewport::{Viewport, FALLBACK_HEIGHT, FALLBACK_WIDTH};

mod addsub;
mod autoindent;
mod buffers;
mod case_ops;
mod changes;
mod cmd_complete;
mod copy_char;
mod gutter;
mod hscroll;
mod indent;
mod insert_erase;
mod insert_shift;
mod join;
mod jumps;
mod line_ops;
mod marks;
mod match_ops;
mod motions;
mod operators;
mod paste;
mod preview;
mod prompt_history;
mod registers;
mod repeat;
mod replace;
mod replace_mode;
mod scroll;
mod search_mode;
mod sent_ops;
mod session;
mod shift_ops;
mod substitute;
mod syntax;
mod tab;
mod visual_block;
mod word_search;
mod workspace_edit;

use session::InsertEdit;

pub use buffers::{BufferError, EditError, WriteOutcome};
pub use prompt_history::{HistoryEntries, DEFAULT_HISTORY_LIMIT, MAX_HISTORY_LIMIT};

pub struct Editor {
    buffer: Buffer,
    cursor: Cursor,
    mode: Mode,
    command: CommandLine,
    search: CommandLine,
    command_history: prompt_history::PromptHistory,
    search_history: prompt_history::PromptHistory,
    search_origin: Option<(usize, usize)>,
    search_status: Option<SearchStatus>,
    selection: Option<Selection>,
    previous_visual: Option<SavedVisual>,
    last_insert: Option<(usize, usize)>,
    insert_entry: Option<(Action, usize, Option<char>)>,
    last_change: Option<repeat::RepeatChange>,
    undone_change: Option<repeat::RepeatChange>,
    file_indent: indent::IndentConfig,
    substitute_pattern: String,
    substitute_replacement: String,
    substitute_settings: substitute::SubstituteSettings,
    command_preview: Option<preview::CommandPreview>,
    shiftwidth_override: Option<usize>,
    expandtab_override: Option<bool>,
    registers: crate::registers::Registers,
    pending_register: Option<char>,
    force_numbered_register: bool,
    clipboard: Option<Box<dyn crate::clipboard::Clipboard>>,
    count_digits: usize,
    operator_count: Option<usize>,
    insert_repeat: Option<usize>,
    insert_reopen: bool,
    insert_start: Option<(usize, usize)>,
    insert_record: Vec<InsertEdit>,
    insert_moved: bool,
    insert_last_char: Option<char>,
    replace_entries: Vec<replace_mode::ReplaceEntry>,
    block_insert: Option<visual_block::BlockInsertPlan>,
    viewport: Viewport,
    jumps: Jumps,
    marks: Marks,
    change_list: Vec<(usize, usize)>,
    change_index: usize,
    insert_session_origin: Option<(usize, usize)>,
    insert_run_origin: Option<(usize, usize)>,
    insert_run_active: bool,
    change_origin_override: Option<(usize, usize)>,
    clipboard_unnamed: bool,
    scroll_amount: Option<usize>,
    number: bool,
    relativenumber: bool,
    numberwidth: usize,
    signcolumn_width: usize,
    ignorecase: bool,
    smartcase: bool,
    search_no_smartcase: bool,
    search_forward: bool,
    search_prompt_forward: bool,
    search_last: String,
    search_view_top: usize,
    completion: Option<cmd_complete::CompletionState>,
    autoindent: bool,
    pending_autoindent: Option<(usize, String)>,
    buffers: buffers::Buffers,
}

struct SearchStatus {
    index: usize,
    total: usize,
    kind: crate::search_stat::MatchStatKind,
    wrapped: bool,
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
                self.pending_register = None;
                return;
            }
            Action::SelectRegister(name) => {
                self.select_register(name);
                return;
            }
            _ => {}
        }
        let was_insert = self.mode.is_insert();
        let change_register = self.pending_register;
        self.force_numbered_register = uses_numbered_register(action);
        let jump = jump_recording(action);
        let jump_before = (self.cursor.row(), self.cursor.col());
        if self.needs_checkpoint(action) {
            self.record_boundary();
        }
        if repeat::is_insert_entry(action) {
            self.insert_entry = Some((action, self.peek_count(), self.pending_register));
        }
        let was_normal = self.mode == Mode::Normal;
        let revision = self.buffer.revision();
        let change_count = self.peek_count();
        self.change_origin_override = None;
        let change_origin = self.change_origin();
        match action {
            Action::MoveUp => self.move_up(),
            Action::MoveDown => self.move_down(),
            Action::MoveLeft => self.move_left(),
            Action::MoveRight => self.move_right(),
            Action::MoveToLineStart => self.move_to_line_start(),
            Action::MoveToFirstNonBlank => self.move_to_first_non_blank(),
            Action::ScrollLineUp => self.scroll_line_up(),
            Action::ScrollLineDown => self.scroll_line_down(),
            Action::ScrollHalfUp => self.scroll_half_up(),
            Action::ScrollHalfDown => self.scroll_half_down(),
            Action::ScrollPageUp => self.scroll_page_up(),
            Action::ScrollPageDown => self.scroll_page_down(),
            Action::ScrollLeft => self.scroll_left(),
            Action::ScrollRight => self.scroll_right(),
            Action::ScrollHalfLeft => self.scroll_half_left(),
            Action::ScrollHalfRight => self.scroll_half_right(),
            Action::ScrollCursorStart => self.scroll_cursor_start(),
            Action::ScrollCursorEnd => self.scroll_cursor_end(),
            Action::MoveToViewportTop => self.move_to_viewport_top(),
            Action::MoveToViewportMiddle => self.move_to_viewport_middle(),
            Action::MoveToViewportBottom => self.move_to_viewport_bottom(),
            Action::SetMark(name) => self.set_mark(name),
            Action::JumpToMark { name, line_wise } => {
                let _ = self.jump_to_mark(name, line_wise);
            }
            Action::JumpToPrevious { line_wise } => {
                let _ = self.jump_to_previous(line_wise);
            }
            Action::DeleteToMark { name, line_wise } => {
                let _ = self.delete_to_mark(name, line_wise);
            }
            Action::ChangeToMark { name, line_wise } => {
                let _ = self.change_to_mark(name, line_wise);
            }
            Action::YankToMark { name, line_wise } => {
                let _ = self.yank_to_mark(name, line_wise);
            }
            Action::JumpOlder => {
                let _ = self.jump_older();
            }
            Action::JumpNewer => {
                let _ = self.jump_newer();
            }
            Action::OlderChange => self.older_change(),
            Action::NewerChange => self.newer_change(),
            Action::EditAlternate => self.buffer_alternate(),
            Action::InsertRegister(name) => self.insert_register(name),
            Action::SelectRegister(_) => self.noop(),
            Action::MoveToMatch => self.move_to_match(),
            Action::SearchWordForward => {
                let _ = self.search_word(true);
            }
            Action::SearchWordBackward => {
                let _ = self.search_word(false);
            }
            Action::SearchBareWordForward => {
                let _ = self.search_word_bare(true);
            }
            Action::SearchBareWordBackward => {
                let _ = self.search_word_bare(false);
            }
            Action::CompleteCommand => self.complete_command(false),
            Action::CompleteCommandBackward => self.complete_command(true),
            Action::DeleteToMatch => self.delete_match(),
            Action::YankToMatch => self.yank_match(),
            Action::ChangeToMatch => self.change_match(),
            Action::MoveToNextSentence => self.move_to_next_sentence(),
            Action::MoveToPrevSentence => self.move_to_prev_sentence(),
            Action::MoveToNextParagraph => self.move_to_next_paragraph(),
            Action::MoveToPrevParagraph => self.move_to_prev_paragraph(),
            Action::DeleteToNextSentence => self.delete_next_sentence(),
            Action::DeleteToPrevSentence => self.delete_prev_sentence(),
            Action::DeleteToNextParagraph => self.delete_next_paragraph(),
            Action::DeleteToPrevParagraph => self.delete_prev_paragraph(),
            Action::YankToNextSentence => self.yank_next_sentence(),
            Action::YankToPrevSentence => self.yank_prev_sentence(),
            Action::YankToNextParagraph => self.yank_next_paragraph(),
            Action::YankToPrevParagraph => self.yank_prev_paragraph(),
            Action::ChangeToNextSentence => self.change_next_sentence(),
            Action::ChangeToPrevSentence => self.change_prev_sentence(),
            Action::ChangeToNextParagraph => self.change_next_paragraph(),
            Action::ChangeToPrevParagraph => self.change_prev_paragraph(),
            Action::PlaceCenter => self.place_center(),
            Action::PlaceTop => self.place_top(),
            Action::PlaceBottom => self.place_bottom(),
            Action::MoveToLineEnd => self.move_to_line_end(),
            Action::MoveToNextWordStart => self.move_to_next_word_start(),
            Action::MoveToPrevWordStart => self.move_to_prev_word_start(),
            Action::MoveToNextWordEnd => self.move_to_next_word_end(),
            Action::MoveToNextLineStart => self.move_to_next_line_start(),
            Action::MoveToPrevLineStart => self.move_to_prev_line_start(),
            Action::InsertChar(c) => self.insert_char(c),
            Action::InsertNewline => self.insert_newline(),
            Action::InsertTab => self.insert_tab(),
            Action::InsertShiftRight => self.insert_shift_right(),
            Action::InsertShiftLeft => self.insert_shift_left(),
            Action::InsertCopyBelow => self.insert_copy_below(),
            Action::InsertCopyAbove => self.insert_copy_above(),
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
            Action::MoveToPrevWordEnd => self.move_to_prev_word_end(),
            Action::DeleteWordEndBackward => self.delete_word_end_backward(),
            Action::ChangeWordEndBackward => self.change_word_end_backward(),
            Action::YankWordEndBackward => self.yank_word_end_backward(),
            Action::YankBigWordEndBackward => self.yank_big_word_end_backward(),
            Action::ReplaceChar(replacement) => self.replace_char(replacement),
            Action::Increment => self.add_to_number(false),
            Action::Decrement => self.add_to_number(true),
            Action::IncrementSequence => self.add_to_number_sequence(false),
            Action::DecrementSequence => self.add_to_number_sequence(true),
            Action::DeleteBefore => self.delete_before(),
            Action::Substitute => self.substitute(),
            Action::OpenLineAbove => self.open_line_above(),
            Action::JoinLines => self.join_lines(false),
            Action::JoinLinesNoSpace => self.join_lines(true),
            Action::InsertBackspaceWord => self.insert_backspace_word(),
            Action::InsertBackspaceLine => self.insert_backspace_line(),
            Action::DeleteBigWordEndBackward => self.delete_big_word_end_backward(),
            Action::ChangeBigWordEndBackward => self.change_big_word_end_backward(),
            Action::YankWord => self.yank_word(),
            Action::YankBigWord => self.yank_big_word(),
            Action::YankWordBackward => self.yank_word_backward(),
            Action::YankBigWordBackward => self.yank_big_word_backward(),
            Action::YankWordEnd => self.yank_word_end(),
            Action::YankBigWordEnd => self.yank_big_word_end(),
            Action::DeleteBigWord => self.delete_big_word(),
            Action::DeleteBigWordBackward => self.delete_big_word_backward(),
            Action::DeleteBigWordEnd => self.delete_big_word_end(),
            Action::ChangeBigWord => self.change_big_word(),
            Action::ChangeBigWordBackward => self.change_big_word_backward(),
            Action::ChangeWordBackward => self.change_word_backward(),
            Action::ChangeBigWordEnd => self.change_big_word_end(),
            Action::MoveToNextBigWordStart => self.move_to_next_big_word_start(),
            Action::MoveToPrevBigWordStart => self.move_to_prev_big_word_start(),
            Action::MoveToNextBigWordEnd => self.move_to_next_big_word_end(),
            Action::MoveToPrevBigWordEnd => self.move_to_prev_big_word_end(),
            Action::LowercaseAt { motion } => self.lowercase_at(motion),
            Action::UppercaseAt { motion } => self.uppercase_at(motion),
            Action::GotoLastLine => self.goto_last_line(),
            Action::GotoFirstLine => self.goto_first_line(),
            Action::DeleteToLastLine => self.delete_to_last_line(),
            Action::DeleteToFirstLine => self.delete_to_first_line(),
            Action::DeleteToViewportTop => self.delete_to_viewport_top(),
            Action::DeleteToViewportMiddle => self.delete_to_viewport_middle(),
            Action::DeleteToViewportBottom => self.delete_to_viewport_bottom(),
            Action::YankToLastLine => self.yank_to_last_line(),
            Action::YankToFirstLine => self.yank_to_first_line(),
            Action::YankToViewportTop => self.yank_to_viewport_top(),
            Action::YankToViewportMiddle => self.yank_to_viewport_middle(),
            Action::YankToViewportBottom => self.yank_to_viewport_bottom(),
            Action::ChangeToLastLine => self.change_to_last_line(),
            Action::ChangeToFirstLine => self.change_to_first_line(),
            Action::ChangeToViewportTop => self.change_to_viewport_top(),
            Action::ChangeToViewportMiddle => self.change_to_viewport_middle(),
            Action::ChangeToViewportBottom => self.change_to_viewport_bottom(),
            Action::ShiftRight => self.shift_right(),
            Action::ShiftLeft => self.shift_left(),
            Action::Reindent => self.reindent(),
            Action::ShiftRightAt { motion } => self.shift_right_at(motion),
            Action::ShiftLeftAt { motion } => self.shift_left_at(motion),
            Action::ReindentAt { motion } => self.reindent_at(motion),
            Action::EnterVisual => self.enter_visual(),
            Action::EnterVisualLine => self.enter_visual_line(),
            Action::EnterVisualBlock => self.enter_visual_block(),
            Action::SwapBlockCorners => self.swap_block_corners(),
            Action::ReplaceSelection(replacement) => self.replace_selection(replacement),
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
            Action::DeleteToFirstNonBlank => self.delete_to_first_non_blank(),
            Action::YankToStart => self.yank_to_start(),
            Action::YankToFirstNonBlank => self.yank_to_first_non_blank(),
            Action::ChangeToStart => self.change_to_start(),
            Action::ChangeToFirstNonBlank => self.change_to_first_non_blank(),
            Action::ChangeLine => self.change_line(),
            Action::ChangeToEnd => self.change_to_end(),
            Action::PasteAfter => self.paste_after(),
            Action::PasteBefore => self.paste_before(),
            Action::PasteAfterAdvance => self.paste_after_advance(),
            Action::PasteBeforeAdvance => self.paste_before_advance(),
            Action::Undo => {
                self.undo();
            }
            Action::Redo => {
                self.redo();
            }
            Action::RepeatLastChange => self.repeat_last_change(),
            Action::EnterInsert => {
                self.arm_insert_repeat(false);
                self.enter_insert();
            }
            Action::EnterReplace => {
                self.arm_insert_repeat(false);
                self.enter_replace();
            }
            Action::InsertAtLast => self.insert_at_last(),
            Action::InsertAtColumnStart => self.insert_at_column_start(),
            Action::ToggleChar => self.toggle_char(),
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
            Action::HistoryOlder => self.history_older(),
            Action::HistoryNewer => self.history_newer(),
            Action::StartMacroRecording(_)
            | Action::RunMacro(_)
            | Action::RepeatLastMacro
            | Action::RepeatRecordedMacro
            | Action::ExecuteLastCommand
            | Action::MacroRegisterError(_) => self.noop(),
            Action::EnterSearch => self.enter_search(true),
            Action::EnterSearchBackward => self.enter_search(false),
            Action::PushSearchChar(c) => self.push_search_char(c),
            Action::DeleteSearchChar => self.delete_search_char(),
            Action::ConfirmSearch => {
                let _ = self.confirm_search();
            }
            Action::CancelSearch => self.cancel_search(),
            Action::NextMatch => {
                self.next_match();
            }
            Action::PrevMatch => {
                self.prev_match();
            }
            Action::Save => self.save_silent(),
            Action::Quit | Action::Noop => self.noop(),
            Action::OpenFilePicker
            | Action::OpenGrepPicker
            | Action::OpenGrepWordPicker
            | Action::OpenBufferPicker
            | Action::OpenReplacePicker
            | Action::GitNextHunk
            | Action::GitPrevHunk
            | Action::GitStageHunk
            | Action::GitUndoStage
            | Action::GitStageBuffer
            | Action::GitPreviewHunk
            | Action::GitDiff
            | Action::DiagnosticNext
            | Action::DiagnosticPrev
            | Action::GotoDefinition
            | Action::ShowReferences
            | Action::Hover
            | Action::Rename => self.noop(),
            Action::PushCount(_) | Action::BeginSequence | Action::CancelPending => self.noop(),
        }
        if was_insert {
            self.insert_last_char = match action {
                Action::InsertChar(c) => Some(c),
                _ => None,
            };
        }
        self.pending_register = None;
        self.force_numbered_register = false;
        self.normalize_cursor();
        self.track_head(action);
        self.clear_count();
        if jump_records(
            jump,
            jump_before,
            (self.cursor.row(), self.cursor.col()),
            change_count,
        ) {
            self.set_pcmark_at(jump_before);
        }
        self.apply_line_adjustments();
        self.refresh_wanted_column(action);
        self.reveal_cursor();
        if !matches!(action, Action::ScrollCursorStart | Action::ScrollCursorEnd) {
            self.reveal_cursor_horizontally();
        }
        let buffer_changed = self.buffer.revision() != revision;
        if was_insert && Self::breaks_insert_run(action) {
            self.insert_run_active = false;
        }
        if was_normal
            && !matches!(
                action,
                Action::Undo | Action::Redo | Action::RepeatLastChange
            )
            && buffer_changed
        {
            self.record_change(repeat::RepeatChange::Edit {
                action,
                count: change_count,
                register: change_register,
                record: Vec::new(),
            });
        }
        if buffer_changed && !matches!(action, Action::Undo | Action::Redo) {
            let insert_entry =
                !was_insert && repeat::is_insert_entry(action) && self.mode.is_insert();
            let change_origin = if was_insert && action == Action::DeleteBackward {
                (change_origin.0, change_origin.1.saturating_sub(1))
            } else {
                change_origin
            };
            self.note_buffer_change(change_origin, was_insert, insert_entry);
            if action == Action::InsertNewline {
                self.insert_run_active = false;
            }
        }
        if self.mode == Mode::Command {
            self.refresh_command_preview();
        } else {
            self.clear_command_preview();
        }
        self.change_origin_override = None;
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

    pub fn virtcol(&self) -> usize {
        let row = self.cursor.row();
        let col = self.cursor.col();
        let tabstop = self.effective_indent().tabstop;
        let width = self.buffer.line_len(row);
        if col < width {
            self.buffer
                .screen_col(row, col.saturating_add(1), tabstop)
                .saturating_sub(1)
        } else {
            self.buffer.screen_col(row, width, tabstop)
        }
    }

    pub fn cursor_screen_col(&self) -> usize {
        let row = self.cursor.row();
        let col = self.cursor.col();
        let tabstop = self.effective_indent().tabstop;
        if self.cursor_shows_tab_end(row, col) {
            return self
                .buffer
                .screen_col(row, col.saturating_add(1), tabstop)
                .saturating_sub(1);
        }
        self.buffer.screen_col(row, col, tabstop)
    }

    pub(super) fn remember_wanted_column(&mut self) {
        self.cursor.set_desired(self.cursor_screen_col());
    }

    fn refresh_wanted_column(&mut self, action: Action) {
        if action.resets_wanted_column() {
            self.remember_wanted_column();
        }
    }

    fn cursor_shows_tab_end(&self, row: usize, col: usize) -> bool {
        self.mode.is_normal_state()
            && self.buffer.char_at(row, col) == Some('\t')
            && !self.cursor_at_selection_start(row, col)
    }

    fn cursor_at_selection_start(&self, row: usize, col: usize) -> bool {
        self.selection
            .as_ref()
            .is_some_and(|selection| selection.start() == (row, col))
    }

    pub fn cursor_line_empty(&self) -> bool {
        self.buffer.line_len(self.cursor.row()) == 0
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn set_shiftwidth(&mut self, width: usize) {
        self.shiftwidth_override = Some(width);
    }

    pub fn set_expandtab(&mut self, expandtab: bool) {
        self.expandtab_override = Some(expandtab);
    }

    pub(super) fn effective_indent(&self) -> indent::IndentConfig {
        indent::IndentConfig {
            shiftwidth: self
                .shiftwidth_override
                .unwrap_or(self.file_indent.shiftwidth),
            expandtab: self
                .expandtab_override
                .unwrap_or(self.file_indent.expandtab),
            tabstop: self.file_indent.tabstop,
        }
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
        let file_indent = indent::detect(buffer.path.as_deref());
        Self {
            buffer,
            cursor: Cursor::new(),
            mode: Mode::Normal,
            command: CommandLine::new(),
            search: CommandLine::new(),
            command_history: prompt_history::PromptHistory::new(DEFAULT_HISTORY_LIMIT),
            search_history: prompt_history::PromptHistory::new(DEFAULT_HISTORY_LIMIT),
            search_origin: None,
            search_status: None,
            selection: None,
            previous_visual: None,
            last_insert: None,
            insert_entry: None,
            last_change: None,
            undone_change: None,
            file_indent,
            substitute_pattern: String::new(),
            substitute_replacement: String::new(),
            substitute_settings: substitute::SubstituteSettings::default(),
            command_preview: None,
            shiftwidth_override: None,
            expandtab_override: None,
            registers: crate::registers::Registers::new(),
            pending_register: None,
            force_numbered_register: false,
            clipboard: None,
            count_digits: 0,
            operator_count: None,
            insert_repeat: None,
            insert_reopen: false,
            insert_start: None,
            insert_record: Vec::new(),
            insert_moved: false,
            insert_last_char: None,
            replace_entries: Vec::new(),
            block_insert: None,
            viewport: Viewport::new(FALLBACK_HEIGHT, FALLBACK_WIDTH),
            jumps: Jumps::new(),
            marks: Marks::new(),
            change_list: Vec::new(),
            change_index: 0,
            insert_session_origin: None,
            insert_run_origin: None,
            insert_run_active: false,
            change_origin_override: None,
            clipboard_unnamed: true,
            scroll_amount: None,
            number: true,
            relativenumber: false,
            numberwidth: 4,
            signcolumn_width: 0,
            ignorecase: false,
            smartcase: false,
            search_no_smartcase: false,
            search_forward: true,
            search_prompt_forward: true,
            search_last: String::new(),
            search_view_top: 0,
            completion: None,
            autoindent: true,
            pending_autoindent: None,
            buffers: buffers::Buffers::new(),
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

    pub(super) fn peek_count(&self) -> usize {
        let operator = self.operator_count.unwrap_or(1);
        if self.count_digits == 0 {
            return operator;
        }
        operator.saturating_mul(self.count_digits).min(999_999_999)
    }

    pub fn take_count(&mut self) -> usize {
        let operator = self.operator_count.take().unwrap_or(1);
        let pending = std::mem::replace(&mut self.count_digits, 0);
        if pending == 0 {
            return operator;
        }
        operator.saturating_mul(pending).min(999_999_999)
    }

    pub(super) fn has_post_digits(&self) -> bool {
        self.count_digits != 0
    }

    pub(super) fn take_count0(&mut self) -> usize {
        std::mem::replace(&mut self.count_digits, 0)
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
        self.registers
            .unnamed_slot()
            .and_then(|slot| self.registers.get(slot))
            .map(|register| register.kind())
            .unwrap_or(RegisterKind::CharWise)
    }

    pub fn store_macro_register(&mut self, name: char, text: String) {
        self.registers.store_macro(name, text);
    }

    pub fn macro_register(&self, name: char) -> Option<(&str, RegisterKind)> {
        self.registers.macro_register(name)
    }

    #[cfg(test)]
    pub fn register_text(&self) -> String {
        self.registers
            .unnamed_slot()
            .and_then(|slot| self.registers.get(slot))
            .map(|register| register.text().to_string())
            .unwrap_or_default()
    }

    pub fn row(&self) -> usize {
        self.cursor.row()
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

    pub fn search_forward(&self) -> bool {
        self.search_forward
    }

    pub fn search_prompt_delimiter(&self) -> char {
        if self.search_prompt_forward {
            '/'
        } else {
            '?'
        }
    }

    pub fn completion_view(&self) -> Option<(&[String], Option<usize>)> {
        let state = self.completion.as_ref()?;
        Some((state.items(), state.selected()))
    }

    pub fn search_status_text(&self) -> Option<String> {
        let status = self.search_status.as_ref()?;
        if status.revision != self.buffer.revision() {
            return None;
        }
        Some(search_mode::status_line(
            self.search_forward,
            self.search.text(),
            status,
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

    pub fn text_lines(&self) -> Vec<String> {
        self.buffer
            .window_lines(0, self.buffer.line_count().saturating_add(1))
    }

    pub fn document_snapshot(&self) -> Rope {
        self.buffer.snapshot()
    }

    pub fn text_has_final_newline(&self) -> bool {
        self.buffer.has_final_newline()
    }

    fn track_head(&mut self, action: Action) {
        if !is_tracking(action) {
            return;
        }
        self.sync_head();
    }

    pub fn visible_lines(&self, top: usize, height: usize) -> Vec<String> {
        let mut lines = self.buffer.window_lines(top, height);
        if let Some(preview) = self.command_preview.as_ref() {
            for (row, text) in &preview.lines {
                if *row >= top && *row < top.saturating_add(height) {
                    if let Some(line) = lines.get_mut(row - top) {
                        *line = text.clone();
                    }
                }
            }
        }
        lines
    }
}

enum JumpRecording {
    Never,
    Always,
    IfMovedOrSingle,
}

fn uses_numbered_register(action: Action) -> bool {
    matches!(
        action,
        Action::DeleteToMark { .. }
            | Action::ChangeToMark { .. }
            | Action::DeleteToMatch
            | Action::ChangeToMatch
            | Action::DeleteToPrevSentence
            | Action::DeleteToNextSentence
            | Action::ChangeToPrevSentence
            | Action::ChangeToNextSentence
            | Action::DeleteToPrevParagraph
            | Action::DeleteToNextParagraph
            | Action::ChangeToPrevParagraph
            | Action::ChangeToNextParagraph
    )
}

fn jump_recording(action: Action) -> JumpRecording {
    match action {
        Action::GotoLastLine
        | Action::GotoFirstLine
        | Action::MoveToViewportTop
        | Action::MoveToViewportMiddle
        | Action::MoveToViewportBottom
        | Action::DeleteToViewportTop
        | Action::DeleteToViewportMiddle
        | Action::DeleteToViewportBottom
        | Action::YankToViewportTop
        | Action::YankToViewportMiddle
        | Action::YankToViewportBottom
        | Action::ChangeToViewportTop
        | Action::ChangeToViewportMiddle
        | Action::ChangeToViewportBottom
        | Action::DeleteToLastLine
        | Action::DeleteToFirstLine
        | Action::YankToLastLine
        | Action::YankToFirstLine
        | Action::ChangeToLastLine
        | Action::ChangeToFirstLine
        | Action::ShiftRightAt {
            motion: ShiftMotion::GotoLine { .. },
        }
        | Action::ShiftLeftAt {
            motion: ShiftMotion::GotoLine { .. },
        }
        | Action::ReindentAt {
            motion: ShiftMotion::GotoLine { .. },
        }
        | Action::ToggleCaseAt {
            motion: CaseMotion::GotoLine { .. },
        }
        | Action::LowercaseAt {
            motion: CaseMotion::GotoLine { .. },
        }
        | Action::UppercaseAt {
            motion: CaseMotion::GotoLine { .. },
        } => JumpRecording::Always,
        Action::MoveToNextSentence
        | Action::MoveToPrevSentence
        | Action::DeleteToNextSentence
        | Action::DeleteToPrevSentence
        | Action::YankToNextSentence
        | Action::YankToPrevSentence
        | Action::ChangeToNextSentence
        | Action::ChangeToPrevSentence
        | Action::MoveToNextParagraph
        | Action::MoveToPrevParagraph
        | Action::DeleteToNextParagraph
        | Action::DeleteToPrevParagraph
        | Action::YankToNextParagraph
        | Action::YankToPrevParagraph
        | Action::ChangeToNextParagraph
        | Action::ChangeToPrevParagraph
        | Action::DeleteToMark { .. }
        | Action::YankToMark { .. }
        | Action::ChangeToMark { .. } => JumpRecording::IfMovedOrSingle,
        Action::JumpToMark { .. } => JumpRecording::Always,
        _ => JumpRecording::Never,
    }
}

fn jump_records(
    recording: JumpRecording,
    before: (usize, usize),
    after: (usize, usize),
    count: usize,
) -> bool {
    match recording {
        JumpRecording::Never => false,
        JumpRecording::Always => true,
        JumpRecording::IfMovedOrSingle => before != after || count <= 1,
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
            | Action::MoveToFirstNonBlank
            | Action::ScrollLineUp
            | Action::ScrollLineDown
            | Action::ScrollHalfUp
            | Action::ScrollHalfDown
            | Action::ScrollPageUp
            | Action::ScrollPageDown
            | Action::MoveToViewportTop
            | Action::MoveToViewportMiddle
            | Action::MoveToViewportBottom
            | Action::MoveToMatch
            | Action::MoveToNextSentence
            | Action::MoveToPrevSentence
            | Action::MoveToNextParagraph
            | Action::MoveToPrevParagraph
            | Action::MoveToLineEnd
            | Action::MoveToNextWordStart
            | Action::MoveToPrevWordStart
            | Action::MoveToNextWordEnd
            | Action::MoveToPrevWordEnd
            | Action::MoveToNextBigWordStart
            | Action::MoveToPrevBigWordStart
            | Action::MoveToNextBigWordEnd
            | Action::MoveToPrevBigWordEnd
            | Action::FindChar { .. }
            | Action::RepeatFind { .. }
            | Action::GotoLastLine
            | Action::GotoFirstLine
            | Action::JumpTo(_, _)
    )
}

#[cfg(test)]
mod ruler_tests {
    use super::*;
    use crate::actions::Action;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn virtcol_on_tab_spans_to_tabstop() {
        let mut e = editor_with("\tello world");
        e.apply(Action::JumpTo(0, 0));
        assert_eq!(e.col(), 0);
        assert_eq!(e.virtcol(), 7);
        e.apply(Action::JumpTo(0, 1));
        assert_eq!(e.virtcol(), 8);
    }

    #[test]
    fn virtcol_matches_column_for_ascii() {
        let mut e = editor_with("abc");
        e.apply(Action::JumpTo(0, 2));
        assert_eq!(e.col(), 2);
        assert_eq!(e.virtcol(), 2);
    }

    #[test]
    fn cursor_screen_col_matches_column_for_ascii() {
        let mut e = editor_with("abc");
        e.apply(Action::JumpTo(0, 2));
        assert_eq!(e.cursor_screen_col(), 2);
    }

    #[test]
    fn cursor_screen_col_shows_tab_end_in_normal_mode() {
        let mut e = editor_with("\tello world");
        e.apply(Action::JumpTo(0, 0));
        assert_eq!(e.cursor_screen_col(), 7);
    }

    #[test]
    fn cursor_screen_col_starts_tab_in_insert_mode() {
        let mut e = editor_with("\tello world");
        e.apply(Action::InsertAtColumnStart);
        assert_eq!(e.col(), 0);
        assert_eq!(e.cursor_screen_col(), 0);
    }

    #[test]
    fn cursor_screen_col_at_selection_start_starts_tab() {
        let mut e = editor_with("\tello world");
        e.apply(Action::JumpTo(0, 0));
        e.apply(Action::EnterVisual);
        assert_eq!(e.cursor_screen_col(), 0);
    }

    #[test]
    fn cursor_screen_col_after_selection_start_shows_tab_end() {
        let mut e = editor_with("a\tbc");
        e.apply(Action::JumpTo(0, 0));
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        assert_eq!(e.col(), 1);
        assert_eq!(e.cursor_screen_col(), 7);
    }

    #[test]
    fn cursor_screen_col_at_insert_line_end_is_line_width() {
        let mut e = editor_with("\tab");
        e.apply(Action::JumpTo(0, 1));
        e.apply(Action::AppendLineEnd);
        assert_eq!(e.cursor_screen_col(), 10);
    }

    #[test]
    fn virtcol_at_line_end_is_line_width() {
        let mut e = editor_with("abc");
        e.apply(Action::JumpTo(0, 2));
        e.apply(Action::Append);
        assert_eq!(e.col(), 3);
        assert_eq!(e.virtcol(), 3);
    }

    #[test]
    fn cursor_line_empty_detects_blank_line() {
        let mut e = editor_with("ab\n\ncd");
        e.apply(Action::JumpTo(1, 0));
        assert!(e.cursor_line_empty());
        e.apply(Action::JumpTo(0, 0));
        assert!(!e.cursor_line_empty());
    }

    #[test]
    fn is_empty_detects_empty_buffer() {
        let e = editor_with("");
        assert!(e.is_empty());
        let e = editor_with("a");
        assert!(!e.is_empty());
    }
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
