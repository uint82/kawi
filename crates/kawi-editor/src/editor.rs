use anyhow::Result;
use kawi_buffer::Buffer;

use crate::actions::Action;
use crate::command::CommandLine;
use crate::cursor::Cursor;
use crate::mode::Mode;
use crate::register::{Register, RegisterKind};
use crate::search::{self, SearchOutcome};
use crate::selection::Selection;
use crate::words;

pub struct Editor {
    buffer: Buffer,
    cursor: Cursor,
    mode: Mode,
    command: CommandLine,
    search: CommandLine,
    search_origin: Option<(usize, usize)>,
    search_status: Option<SearchStatus>,
    selection: Option<Selection>,
    register: Register,
}

struct SearchStatus {
    pattern: String,
    index: usize,
    total: usize,
    revision: u64,
    current: (usize, usize),
}

impl Editor {
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
            register: Register::new(),
        }
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn row(&self) -> usize {
        self.cursor.row()
    }

    pub fn col(&self) -> usize {
        self.cursor.col()
    }

    pub fn is_dirty(&self) -> bool {
        self.buffer.is_dirty()
    }

    pub fn line_count(&self) -> usize {
        self.buffer.line_count()
    }

    pub fn visible_lines(&self, top: usize, height: usize) -> Vec<String> {
        self.buffer.window_lines(top, height)
    }

    #[cfg(test)]
    pub fn text_lines(&self) -> Vec<String> {
        self.buffer
            .window_lines(0, self.buffer.line_count().saturating_add(1))
    }

    pub fn filename(&self) -> &str {
        self.buffer.path.as_deref().unwrap_or("[No Name]")
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

    pub fn search_prompt(&self) -> Option<&str> {
        if self.mode == Mode::Search {
            Some(self.search.text())
        } else {
            None
        }
    }

    pub fn search_pattern(&self) -> &str {
        self.search.text()
    }

    pub fn search_status_text(&self) -> Option<String> {
        let status = self.search_status.as_ref()?;
        if status.revision != self.buffer.revision() {
            return None;
        }
        Some(status_line(self.search.text(), status.index, status.total))
    }

    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection
            .as_ref()
            .map(|selection| (selection.start(), selection.end()))
    }

    pub fn save(&mut self) -> Result<()> {
        self.buffer.save()
    }

    pub fn byte_count(&self) -> usize {
        self.buffer.byte_count()
    }

    pub fn apply(&mut self, action: Action) {
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
            Action::DeleteSelection => self.delete_selection(),
            Action::YankSelection => self.yank_selection(),
            Action::YankLine => self.yank_line(),
            Action::DeleteToEnd => self.delete_to_end(),
            Action::ChangeLine => self.change_line(),
            Action::ChangeToEnd => self.change_to_end(),
            Action::PasteAfter => self.paste_after(),
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::EnterInsert => self.enter_insert(),
            Action::ExitInsert => self.exit_insert(),
            Action::Append => self.append(),
            Action::AppendLineEnd => self.append_line_end(),
            Action::InsertLineStart => self.insert_line_start(),
            Action::InsertBeforeSelection => self.insert_before_selection(),
            Action::InsertAfterSelection => self.insert_after_selection(),
            Action::CollapseSelection => self.collapse_selection(),
            Action::EnterVisual => self.enter_visual(),
            Action::ExitVisual => self.exit_visual(),
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
        }
        self.normalize_cursor();
        self.track_head(action);
    }

    fn track_head(&mut self, action: Action) {
        if !is_tracking(action) {
            return;
        }
        self.sync_head();
    }

    fn move_up(&mut self) {
        self.cursor.move_up();
        self.apply_desired_column();
    }

    fn move_down(&mut self) {
        self.cursor.move_down();
        self.apply_desired_column();
    }

    fn move_left(&mut self) {
        self.cursor.move_left();
    }

    fn move_right(&mut self) {
        self.cursor.move_right();
    }

    fn move_to_line_start(&mut self) {
        self.cursor.move_to_line_start(self.cursor.row());
    }

    fn move_to_next_word_start(&mut self) {
        let (row, col) = words::next_word_start(&self.buffer, self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    fn move_to_prev_word_start(&mut self) {
        let (row, col) = words::prev_word_start(&self.buffer, self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    fn move_to_next_word_end(&mut self) {
        let (row, col) = words::next_word_end(&self.buffer, self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    fn move_to_next_line_start(&mut self) {
        let row = self.cursor.row().saturating_add(1);
        if row > self.buffer.max_cursor_row() {
            return;
        }
        self.jump_to_line_start(row);
    }

    fn move_to_prev_line_start(&mut self) {
        let row = self.cursor.row().saturating_sub(1);
        self.jump_to_line_start(row);
    }

    fn jump_to_line_start(&mut self, row: usize) {
        let row = row.min(self.buffer.max_cursor_row());
        let col = self
            .buffer
            .line_cow(row)
            .map_or(0, |line| line_start_col(&line));
        self.cursor.set_position(row, col);
    }

    fn move_to_line_end(&mut self) {
        let end = self.max_col_for_row(self.cursor.row());
        self.cursor.set_col(end);
        self.remember_line_end();
    }

    fn remember_line_end(&mut self) {
        if self.mode.is_insert() {
            self.cursor.set_desired(self.cursor.col());
        } else {
            self.cursor.set_desired(usize::MAX);
        }
    }

    fn apply_desired_column(&mut self) {
        let col = self
            .cursor
            .desired()
            .min(self.max_col_for_row(self.cursor.row()));
        self.cursor.set_col(col);
    }

    fn insert_char(&mut self, c: char) {
        self.record_maybe();
        self.buffer
            .insert_char(self.cursor.row(), self.cursor.col(), c);
        self.cursor.advance_col();
    }

    fn insert_newline(&mut self) {
        let row = self.cursor.row();
        self.insert_newline_at(row, self.cursor.col());
    }

    fn insert_newline_at(&mut self, row: usize, col: usize) {
        self.record_maybe();
        self.buffer.insert_newline(row, col);
        self.cursor.move_to_line_start(row.saturating_add(1));
    }

    fn delete_backward(&mut self) {
        self.record_maybe();
        let (row, col) = self
            .buffer
            .delete_backward(self.cursor.row(), self.cursor.col());
        self.cursor.set_position(row, col);
    }

    fn delete_forward(&mut self) {
        self.record_maybe();
        self.buffer
            .delete_forward(self.cursor.row(), self.cursor.col());
    }

    fn delete_line(&mut self) {
        self.yank_current_line();
        self.buffer.delete_line(self.cursor.row());
        self.cursor.set_col(0);
    }

    fn delete_word(&mut self) {
        let Some(((start_row, start_col), (end_row, end_col))) =
            words::dw_range(&self.buffer, self.cursor.row(), self.cursor.col())
        else {
            return;
        };
        self.record_boundary_at(start_row, start_col);
        self.yank_range(start_row, start_col, end_row, end_col);
        self.buffer
            .delete_range(start_row, start_col, end_row, end_col);
        self.cursor.set_position(start_row, start_col);
    }

    fn delete_word_backward(&mut self) {
        let target = words::prev_word_start(&self.buffer, self.cursor.row(), self.cursor.col());
        self.delete_span_to(target);
    }

    fn delete_word_end(&mut self) {
        let Some(((start_row, start_col), (end_row, end_col))) =
            words::cw_range(&self.buffer, self.cursor.row(), self.cursor.col())
        else {
            return;
        };
        self.record_boundary_at(start_row, start_col);
        self.yank_range(start_row, start_col, end_row, end_col);
        self.buffer
            .delete_range(start_row, start_col, end_row, end_col);
        self.cursor.set_position(start_row, start_col);
    }

    fn delete_span_to(&mut self, target: (usize, usize)) {
        let start = (self.cursor.row(), self.cursor.col());
        if target == start {
            return;
        }
        let ((from_row, from_col), (to_row, to_col)) = ordered_span(start, target);
        self.record_boundary_at(from_row, from_col);
        self.yank_span(from_row, from_col, to_row, to_col);
        self.buffer.delete_span(from_row, from_col, to_row, to_col);
        self.cursor.set_position(from_row, from_col);
    }

    fn yank_span(&mut self, start_row: usize, start_col: usize, end_row: usize, end_col: usize) {
        let text = self
            .buffer
            .span_text(start_row, start_col, end_row, end_col);
        self.register.set_text(text);
    }

    fn change_word(&mut self) {
        let Some(((start_row, start_col), (end_row, end_col))) =
            words::cw_range(&self.buffer, self.cursor.row(), self.cursor.col())
        else {
            return;
        };
        self.record_boundary_at(start_row, start_col);
        self.yank_range(start_row, start_col, end_row, end_col);
        self.buffer
            .delete_range(start_row, start_col, end_row, end_col);
        self.cursor.set_position(start_row, start_col);
        self.switch_to_insert();
    }

    fn yank_current_line(&mut self) {
        let text = self.buffer.line_text(self.cursor.row());
        self.register.set_lines(vec![text]);
    }

    fn yank_line(&mut self) {
        self.yank_current_line();
    }

    fn delete_to_end(&mut self) {
        if self.line_is_empty_at_cursor() {
            return;
        }
        self.record_boundary();
        self.yank_to_end();
        self.buffer.delete_range(
            self.cursor.row(),
            self.cursor.col(),
            self.cursor.row(),
            self.line_width(self.cursor.row()).saturating_sub(1),
        );
    }

    fn line_is_empty_at_cursor(&self) -> bool {
        let width = self.line_width(self.cursor.row());
        width == 0 || self.cursor.col() >= width
    }

    fn yank_to_end(&mut self) {
        let row = self.cursor.row();
        let text = self.buffer.range_text(
            row,
            self.cursor.col(),
            row,
            self.line_width(row).saturating_sub(1),
        );
        self.register.set_text(text);
    }

    fn change_line(&mut self) {
        self.yank_current_line();
        if self.line_is_empty_at_cursor() {
            self.enter_insert();
            return;
        }
        self.record_boundary();
        self.buffer.clear_line(self.cursor.row());
        self.cursor.set_col(0);
        self.switch_to_insert();
    }

    fn change_to_end(&mut self) {
        if self.line_is_empty_at_cursor() {
            self.enter_insert();
            return;
        }
        self.record_boundary();
        self.yank_to_end();
        self.buffer.delete_range(
            self.cursor.row(),
            self.cursor.col(),
            self.cursor.row(),
            self.line_width(self.cursor.row()).saturating_sub(1),
        );
        self.clamp_col_normal();
        self.switch_to_insert();
    }

    fn clamp_col_normal(&mut self) {
        let width = self.line_width(self.cursor.row());
        self.cursor
            .set_col(self.cursor.col().min(width.saturating_sub(1)));
    }

    fn delete_selection(&mut self) {
        let ((start_row, start_col), (end_row, end_col)) = self.operator_range();
        self.record_boundary_at(start_row, start_col);
        self.yank_range(start_row, start_col, end_row, end_col);
        self.buffer
            .delete_range(start_row, start_col, end_row, end_col);
        self.collapse_to(start_row, start_col);
        self.cursor.set_position(start_row, start_col);
    }

    fn yank_selection(&mut self) {
        let ((start_row, start_col), (end_row, end_col)) = self.operator_range();
        self.yank_range(start_row, start_col, end_row, end_col);
        self.cursor.set_position(start_row, start_col);
    }

    fn operator_range(&self) -> ((usize, usize), (usize, usize)) {
        match self.selection_range() {
            Some(range) => range,
            None => (
                (self.cursor.row(), self.cursor.col()),
                (self.cursor.row(), self.cursor.col()),
            ),
        }
    }

    fn collapse_to(&mut self, row: usize, col: usize) {
        self.selection = Some(Selection::new(row, col));
    }

    fn yank_range(&mut self, start_row: usize, start_col: usize, end_row: usize, end_col: usize) {
        let text = self
            .buffer
            .range_text(start_row, start_col, end_row, end_col);
        self.register.set_text(text);
    }

    fn paste_after(&mut self) {
        if self.register.is_empty() {
            return;
        }
        self.record_boundary();
        match self.register.kind() {
            RegisterKind::CharWise => self.paste_inline(),
            RegisterKind::LineWise => self.paste_lines(),
        }
    }

    fn paste_inline(&mut self) {
        let text = self.register.text().to_string();
        let (row, col) = self
            .buffer
            .paste_after(self.cursor.row(), self.cursor.col(), &text);
        self.cursor.set_position(row, col);
    }

    fn paste_lines(&mut self) {
        let lines: Vec<String> = self.register.text().lines().map(str::to_string).collect();
        let row = self.buffer.paste_lines_below(self.cursor.row(), &lines);
        self.cursor.set_position(row, self.first_non_blank_of(row));
    }

    fn first_non_blank_of(&self, row: usize) -> usize {
        self.buffer
            .line(row)
            .map_or(0, |line| kawi_buffer::first_non_blank(&line))
    }

    fn open_line_below(&mut self) {
        let row = self.cursor.row();
        let width = self.line_width(row);
        self.buffer.begin_change();
        self.insert_newline_at(row, width);
        self.switch_to_insert();
    }

    pub fn try_undo(&mut self) -> bool {
        match self.buffer.undo(self.cursor.row(), self.cursor.col()) {
            Some((row, col)) => self.place_restored(row, col),
            None => false,
        }
    }

    pub fn try_redo(&mut self) -> bool {
        self.buffer
            .redo_keep_cursor(self.cursor.row(), self.cursor.col())
    }

    fn place_restored(&mut self, row: usize, col: usize) -> bool {
        self.cursor.set_position(row, col);
        true
    }

    fn undo(&mut self) {
        let _ = self.try_undo();
    }

    fn redo(&mut self) {
        let _ = self.try_redo();
    }

    fn record_boundary(&mut self) {
        self.buffer.checkpoint(self.cursor.row(), self.cursor.col());
    }

    fn record_boundary_at(&mut self, row: usize, col: usize) {
        self.buffer.checkpoint(row, col);
    }

    fn record_maybe(&mut self) {
        self.buffer
            .maybe_checkpoint(self.cursor.row(), self.cursor.col());
    }

    fn needs_checkpoint(&self, action: Action) -> bool {
        self.mode == Mode::Normal && matches!(action, Action::DeleteForward | Action::DeleteLine)
    }

    fn enter_insert(&mut self) {
        self.buffer.begin_change();
        self.switch_to_insert();
    }

    fn switch_to_insert(&mut self) {
        self.mode = Mode::Insert;
    }

    fn exit_insert(&mut self) {
        if self.mode == Mode::Insert {
            self.buffer.end_change();
            self.retreat_after_insert();
        }
        self.mode = Mode::Normal;
    }

    fn retreat_after_insert(&mut self) {
        if self.cursor.col() > 0 {
            self.cursor.move_left();
        }
    }

    fn append(&mut self) {
        self.cursor.move_right();
        self.enter_insert();
    }

    fn append_line_end(&mut self) {
        let end = self.line_width(self.cursor.row());
        self.cursor.set_position(self.cursor.row(), end);
        self.enter_insert();
    }

    fn insert_line_start(&mut self) {
        let row = self.cursor.row();
        let start = self.buffer.insert_start_col(row);
        self.cursor.set_position(row, start);
        self.enter_insert();
    }

    fn insert_before_selection(&mut self) {
        if let Some(((row, col), _)) = self.selection_range() {
            self.cursor.set_position(row, col);
        }
        self.exit_visual();
        self.enter_insert();
    }

    fn insert_after_selection(&mut self) {
        if let Some((_, (row, col))) = self.selection_range() {
            self.cursor.set_position(row, col);
            self.cursor.move_right();
        }
        self.exit_visual();
        self.enter_insert();
    }

    fn collapse_selection(&mut self) {
        if self.selection.is_some() {
            self.collapse_to(self.cursor.row(), self.cursor.col());
        }
    }

    fn enter_visual(&mut self) {
        self.selection = Some(Selection::new(self.cursor.row(), self.cursor.col()));
        self.mode = Mode::Visual;
    }

    fn exit_visual(&mut self) {
        self.selection = None;
        self.mode = Mode::Normal;
    }

    fn jump_to(&mut self, row: usize, col: usize) {
        self.cursor.set_position(row, col);
    }

    fn sync_head(&mut self) {
        if let Some(selection) = self.selection.as_mut() {
            selection.set_head(self.cursor.row(), self.cursor.col());
        }
    }

    fn enter_command(&mut self) {
        self.mode = Mode::Command;
        self.command.clear();
    }

    fn push_command_char(&mut self, c: char) {
        if self.mode == Mode::Command {
            self.command.push(c);
        }
    }

    fn delete_command_char(&mut self) {
        if self.mode == Mode::Command {
            self.command.pop();
        }
    }

    fn cancel_command(&mut self) {
        self.mode = Mode::Normal;
        self.command.clear();
    }

    fn enter_search(&mut self) {
        self.search_origin = Some((self.cursor.row(), self.cursor.col()));
        self.search.clear();
        self.search_status = None;
        self.mode = Mode::Search;
    }

    fn push_search_char(&mut self, c: char) {
        if self.mode != Mode::Search {
            return;
        }
        self.search.push(c);
        self.jump_to_first_result();
    }

    fn delete_search_char(&mut self) {
        if self.mode != Mode::Search {
            return;
        }
        self.search.pop();
        if self.search.text().is_empty() {
            self.cancel_search();
            return;
        }
        self.jump_to_first_result();
    }

    fn jump_to_first_result(&mut self) {
        let origin = self
            .search_origin
            .unwrap_or((self.cursor.row(), self.cursor.col()));
        if let Some((row, col)) =
            search::find_first_from(&self.buffer, self.search.text(), origin.0, origin.1)
        {
            self.cursor.set_position(row, col);
        }
    }

    pub fn confirm_search(&mut self) -> Option<SearchOutcome> {
        if self.mode == Mode::Search {
            self.mode = Mode::Normal;
        }
        if self.search.text().is_empty() {
            self.search_status = None;
            return None;
        }
        self.refresh_search_status();
        if self.search_status.is_some() {
            return Some(SearchOutcome::Found { wrapped: false });
        }
        Some(SearchOutcome::Missing)
    }

    fn cancel_search(&mut self) {
        if let Some((row, col)) = self.search_origin {
            self.cursor.set_position(row, col);
        }
        self.search_origin = None;
        self.search_status = None;
        self.mode = Mode::Normal;
    }

    fn refresh_search_status(&mut self) {
        let pattern = self.search.text().to_string();
        if pattern.is_empty() {
            self.search_status = None;
            return;
        }
        let revision = self.buffer.revision();
        let total = match &self.search_status {
            Some(status) if status.pattern == pattern && status.revision == revision => {
                status.total
            }
            _ => search::count_matches(&self.buffer, &pattern),
        };
        if total == 0 {
            self.search_status = None;
            return;
        }
        let index = search::match_rank(
            &self.buffer,
            &pattern,
            self.cursor.row(),
            self.cursor.col(),
            total,
        );
        self.search_status = Some(SearchStatus {
            pattern,
            index,
            total,
            revision,
            current: (self.cursor.row(), self.cursor.col()),
        });
    }

    pub fn search_spans(&self, top: usize, height: usize) -> Vec<search::MatchSpan> {
        let (pattern, anchor) = match self.highlight_target() {
            Some(target) => target,
            None => return Vec::new(),
        };
        search::collect_matches(&self.buffer, pattern, top, top.saturating_add(height))
            .into_iter()
            .map(|span| self.mark_current(span, anchor))
            .collect()
    }

    fn highlight_target(&self) -> Option<(&str, (usize, usize))> {
        if self.mode == Mode::Search && !self.search.text().is_empty() {
            return Some((self.search.text(), (self.cursor.row(), self.cursor.col())));
        }
        match &self.search_status {
            Some(status) if status.revision == self.buffer.revision() => {
                Some((self.search.text(), status.current))
            }
            _ => None,
        }
    }

    fn mark_current(
        &self,
        mut span: search::MatchSpan,
        anchor: (usize, usize),
    ) -> search::MatchSpan {
        span.current = (span.row, span.start) == anchor;
        span
    }

    pub fn search_next(&mut self) -> SearchOutcome {
        self.step_search(true)
    }

    pub fn search_prev(&mut self) -> SearchOutcome {
        self.step_search(false)
    }

    fn next_match(&mut self) {
        let _ = self.search_next();
    }

    fn prev_match(&mut self) {
        let _ = self.search_prev();
    }

    fn step_search(&mut self, forward: bool) -> SearchOutcome {
        if self.search.text().is_empty() {
            return SearchOutcome::Missing;
        }
        let found = search_step(
            &self.buffer,
            self.search.text(),
            self.cursor.row(),
            self.cursor.col(),
            forward,
        );
        match found {
            Some(((row, col), wrapped)) => self.place_search_result(row, col, wrapped),
            None => SearchOutcome::Missing,
        }
    }

    fn place_search_result(&mut self, row: usize, col: usize, wrapped: bool) -> SearchOutcome {
        self.cursor.set_position(row, col);
        self.refresh_search_status();
        SearchOutcome::Found { wrapped }
    }

    fn save_silent(&mut self) {
        let _ = self.buffer.save();
    }

    fn noop(&mut self) {}

    fn normalize_cursor(&mut self) {
        self.clamp_row();
        self.clamp_col();
    }

    fn clamp_row(&mut self) {
        let max_row = self.buffer.max_cursor_row();
        self.cursor.set_row(self.cursor.row().min(max_row));
    }

    fn clamp_col(&mut self) {
        let max_col = self.max_col_for_row(self.cursor.row());
        self.cursor.set_col(self.cursor.col().min(max_col));
    }

    fn max_col_for_row(&self, row: usize) -> usize {
        let width = self.line_width(row);
        if self.mode.is_insert() {
            width
        } else {
            width.saturating_sub(1)
        }
    }

    fn line_width(&self, row: usize) -> usize {
        self.buffer.line_len(row)
    }
}

fn search_step(
    buffer: &Buffer,
    pattern: &str,
    row: usize,
    col: usize,
    forward: bool,
) -> Option<((usize, usize), bool)> {
    if forward {
        search::find_next(buffer, pattern, row, col)
    } else {
        search::find_prev(buffer, pattern, row, col)
    }
}

fn ordered_span(first: (usize, usize), second: (usize, usize)) -> ((usize, usize), (usize, usize)) {
    if first <= second {
        (first, second)
    } else {
        (second, first)
    }
}

fn line_start_col(line: &str) -> usize {
    match line.chars().enumerate().find(|(_, ch)| !ch.is_whitespace()) {
        Some((idx, _)) => idx,
        None => line.chars().count().saturating_sub(1),
    }
}

fn status_line(pattern: &str, index: usize, total: usize) -> String {
    format!("/{pattern} [{index}/{total}]")
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
            | Action::JumpTo(_, _)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn starts_in_normal() {
        let e = editor_with("hi");
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn enter_and_exit_insert() {
        let mut e = editor_with("hi");
        e.apply(Action::EnterInsert);
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::ExitInsert);
        assert_eq!(e.mode(), Mode::Normal);
    }

    #[test]
    fn exit_insert_steps_back() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterInsert);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ExitInsert);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn append_then_esc_returns_to_origin() {
        let mut e = editor_with("Sbcdef");
        e.apply(Action::Append);
        assert_eq!((e.row(), e.col()), (0, 1));
        e.apply(Action::ExitInsert);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn exit_insert_at_start_stays() {
        let mut e = editor_with("abc");
        e.apply(Action::EnterInsert);
        e.apply(Action::ExitInsert);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn insert_char_advances() {
        let mut e = editor_with("ac");
        e.apply(Action::MoveRight);
        e.apply(Action::InsertChar('b'));
        assert_eq!(e.text_lines()[0], "abc");
        assert_eq!(e.col(), 2);
    }

    #[test]
    fn hjkl_move() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 1));
        e.apply(Action::MoveLeft);
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn line_start_end() {
        let mut e = editor_with("abc");
        e.apply(Action::MoveToLineEnd);
        assert_eq!(e.col(), 2);
        e.apply(Action::MoveToLineStart);
        assert_eq!(e.col(), 0);
    }

    #[test]
    fn enter_lands_on_next_first_content() {
        let mut e = editor_with("  ab\n    cd\nef");
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 4));
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn enter_ignores_start_column() {
        let mut e = editor_with("ab\n    cd");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 4));
    }

    #[test]
    fn enter_on_empty_line_stays_at_start() {
        let mut e = editor_with("ab\n\ncd");
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn enter_on_whitespace_line_stops_at_end() {
        let mut e = editor_with("ab\n   \ncd");
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 2));
    }

    #[test]
    fn enter_on_last_line_stays() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::MoveToNextLineStart);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (1, 1));
    }

    #[test]
    fn minus_moves_to_prev_line_start() {
        let mut e = editor_with("ab\n   \ncd");
        e.apply(Action::MoveToNextLineStart);
        e.apply(Action::MoveToNextLineStart);
        assert_eq!((e.row(), e.col()), (2, 0));
        e.apply(Action::MoveToPrevLineStart);
        assert_eq!((e.row(), e.col()), (1, 2));
        e.apply(Action::MoveToPrevLineStart);
        assert_eq!((e.row(), e.col()), (0, 0));
        e.apply(Action::MoveToPrevLineStart);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn vertical_keeps_goal_column() {
        let mut e = editor_with("0123456789\nabcd\nabcdefghi\n0123456789AB");
        for _ in 0..9 {
            e.apply(Action::MoveRight);
        }
        assert_eq!((e.row(), e.col()), (0, 9));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 3));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (2, 8));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (3, 9));
    }

    #[test]
    fn vertical_up_restores_goal_column() {
        let mut e = editor_with("0123456789\nabcd\nabcdefghi\n0123456789AB");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        for _ in 0..9 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (2, 8));
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (1, 3));
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (0, 9));
    }

    #[test]
    fn line_end_sticks_to_ends() {
        let mut e = editor_with("0123456789\nabcd\nabcdefghi\n0123456789AB");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 3));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (2, 8));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (3, 11));
    }

    #[test]
    fn line_start_clears_sticky_end() {
        let mut e = editor_with("0123456789\nabcd\nabcdefghi\n0123456789AB");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::MoveToLineStart);
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (1, 0));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (2, 0));
        e.apply(Action::MoveDown);
        assert_eq!((e.row(), e.col()), (3, 0));
    }

    #[test]
    fn normal_stays_on_last_char() {
        let mut e = editor_with("abcdef");
        for _ in 0..10 {
            e.apply(Action::MoveRight);
        }
        assert_eq!(e.col(), 5);
    }

    #[test]
    fn insert_reaches_line_end() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterInsert);
        for _ in 0..10 {
            e.apply(Action::MoveRight);
        }
        assert_eq!(e.col(), 6);
    }

    #[test]
    fn exit_insert_pulls_back() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterInsert);
        e.apply(Action::MoveToLineEnd);
        assert_eq!(e.col(), 6);
        e.apply(Action::ExitInsert);
        assert_eq!(e.col(), 5);
    }

    #[test]
    fn empty_line_stays_zero() {
        let mut e = editor_with("");
        e.apply(Action::MoveRight);
        assert_eq!(e.col(), 0);
        e.apply(Action::EnterInsert);
        e.apply(Action::MoveRight);
        assert_eq!(e.col(), 0);
    }

    #[test]
    fn delete_forward_removes() {
        let mut e = editor_with("ab");
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines()[0], "b");
    }

    #[test]
    fn delete_line_removes_row() {
        let mut e = editor_with("a\nb\nc");
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteLine);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[1], "c");
    }

    #[test]
    fn open_line_below_enters_insert() {
        let mut e = editor_with("ab");
        e.apply(Action::OpenLineBelow);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (1, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn command_buffer_collects_text() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterCommand);
        assert_eq!(e.mode(), Mode::Command);
        e.apply(Action::PushCommandChar('w'));
        e.apply(Action::PushCommandChar('q'));
        assert_eq!(e.command_prompt(), Some("wq"));
    }

    #[test]
    fn command_backspace_shrinks() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterCommand);
        e.apply(Action::PushCommandChar('w'));
        e.apply(Action::DeleteCommandChar);
        assert_eq!(e.command_prompt(), Some(""));
    }

    #[test]
    fn command_cancel_returns_to_normal() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterCommand);
        e.apply(Action::PushCommandChar('w'));
        e.apply(Action::CancelCommand);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!(e.command_prompt(), None);
    }

    #[test]
    fn search_typing_jumps_live() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::EnterSearch);
        assert_eq!(e.mode(), Mode::Search);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn search_confirm_stays_put() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn search_cancel_restores_origin() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::CancelSearch);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn search_backspace_empty_cancels() {
        let mut e = editor_with("foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::DeleteSearchChar);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!(e.search_prompt(), None);
    }

    #[test]
    fn search_next_reports_wrap() {
        let mut e = editor_with("ar ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
        assert_eq!((e.row(), e.col()), (0, 3));
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: true });
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn search_prev_wraps_around() {
        let mut e = editor_with("ar ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_prev(), SearchOutcome::Found { wrapped: true });
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn search_without_pattern_is_missing() {
        let mut e = editor_with("foo");
        assert_eq!(e.search_next(), SearchOutcome::Missing);
        assert_eq!(e.search_prev(), SearchOutcome::Missing);
    }

    #[test]
    fn search_confirm_shows_status() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        assert_eq!(
            e.confirm_search(),
            Some(SearchOutcome::Found { wrapped: false })
        );
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [1/1]"));
    }

    #[test]
    fn search_confirm_empty_is_silent() {
        let mut e = editor_with("foo");
        e.apply(Action::EnterSearch);
        assert_eq!(e.confirm_search(), None);
        assert_eq!(e.search_status_text(), None);
    }

    #[test]
    fn search_confirm_without_match_is_missing() {
        let mut e = editor_with("foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('z'));
        assert_eq!(e.confirm_search(), Some(SearchOutcome::Missing));
        assert_eq!(e.search_status_text(), None);
    }

    #[test]
    fn search_next_refreshes_status() {
        let mut e = editor_with("ar ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [1/3]"));
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [2/3]"));
        assert_eq!(e.search_prev(), SearchOutcome::Found { wrapped: false });
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [1/3]"));
    }

    #[test]
    fn search_status_hides_after_edit() {
        let mut e = editor_with("ar ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('x'));
        assert_eq!(e.search_status_text(), None);
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [2/3]"));
    }

    #[test]
    fn search_status_recaches_new_pattern() {
        let mut e = editor_with("ar foo ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [1/2]"));
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('f'));
        e.apply(Action::PushSearchChar('o'));
        e.apply(Action::PushSearchChar('o'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_status_text().as_deref(), Some("/foo [1/1]"));
        assert_eq!(e.search_next(), SearchOutcome::Missing);
        assert_eq!(e.search_status_text().as_deref(), Some("/foo [1/1]"));
    }

    #[test]
    fn search_spans_mark_current_match() {
        let mut e = editor_with("ar ar\nar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        let spans = e.search_spans(0, 2);
        assert_eq!(spans.len(), 3);
        assert_eq!(
            spans.iter().map(|span| span.current).collect::<Vec<_>>(),
            vec![true, false, false]
        );
        e.apply(Action::NextMatch);
        let spans = e.search_spans(0, 2);
        assert_eq!(
            spans.iter().map(|span| span.current).collect::<Vec<_>>(),
            vec![false, true, false]
        );
    }

    #[test]
    fn search_spans_follow_live_typing() {
        let mut e = editor_with("ar ar\nar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        let spans = e.search_spans(0, 2);
        assert_eq!(spans.len(), 3);
        assert!(spans[0].current);
    }

    #[test]
    fn search_spans_empty_without_pattern() {
        let e = editor_with("ar ar");
        assert!(e.search_spans(0, 2).is_empty());
    }

    #[test]
    fn search_spans_clear_on_cancel() {
        let mut e = editor_with("ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_spans(0, 1).len(), 2);
        e.apply(Action::EnterSearch);
        e.apply(Action::CancelSearch);
        assert!(e.search_spans(0, 1).is_empty());
    }

    #[test]
    fn search_cancel_clears_status() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        e.apply(Action::EnterSearch);
        e.apply(Action::CancelSearch);
        assert_eq!(e.search_status_text(), None);
    }

    #[test]
    fn insert_session_undoes_together() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::InsertChar('Z'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn empty_insert_leaves_no_history() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::ExitInsert);
        let before = e.text_lines()[0].to_string();
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], before);
    }

    #[test]
    fn open_line_session_undoes_together() {
        let mut e = editor_with("ab");
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('H'));
        e.apply(Action::InsertChar('i'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn redo_restores_undone_session() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        e.apply(Action::Redo);
        assert_eq!(e.text_lines()[0], "Xab");
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut e = editor_with("ab");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        e.apply(Action::DeleteForward);
        e.apply(Action::Redo);
        assert_eq!(e.text_lines()[0], "b");
    }

    #[test]
    fn undo_open_line_returns_cursor() {
        let mut e = editor_with("aa\nbb");
        e.apply(Action::MoveDown);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('H'));
        e.apply(Action::InsertChar('i'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn undo_session_returns_to_start() {
        let mut e = editor_with("aa");
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('X'));
        e.apply(Action::InsertChar('Y'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "aa");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn undo_delete_returns_cursor() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteForward);
        assert_eq!(e.text_lines()[0], "abdef");
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn redo_keeps_cursor() {
        let mut e = editor_with("aa\nbb");
        e.apply(Action::MoveDown);
        e.apply(Action::OpenLineBelow);
        e.apply(Action::InsertChar('H'));
        e.apply(Action::InsertChar('i'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!((e.row(), e.col()), (1, 0));
        e.apply(Action::Redo);
        assert_eq!(e.text_lines()[2], "Hi");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn visual_anchors_at_entry() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 1))));
    }

    #[test]
    fn visual_motion_extends_head() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn visual_exit_keeps_head_clears() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ExitVisual);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!(e.selection_range(), None);
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn visual_motion_down_extends_rows() {
        let mut e = editor_with("abcdef\nghijkl");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        assert_eq!(e.selection_range(), Some(((0, 0), (1, 0))));
    }

    #[test]
    fn visual_delete_cuts_range() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.text_lines()[0], "aef");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 1))));
    }

    #[test]
    fn delete_selection_undoes_together() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "abcdef");
    }

    #[test]
    fn undo_multiline_delete_returns_to_start() {
        let mut e = editor_with("ab\ncd");
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteSelection);
        assert_eq!(e.text_lines().len(), 1);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn yank_line_pastes_below() {
        let mut e = editor_with("aa\nbb\ncc");
        e.apply(Action::MoveDown);
        e.apply(Action::YankLine);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines().len(), 4);
        assert_eq!(e.text_lines()[2], "bb");
        assert_eq!((e.row(), e.col()), (2, 0));
    }

    #[test]
    fn delete_line_yanks_for_paste() {
        let mut e = editor_with("aa\nbb");
        e.apply(Action::DeleteLine);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[0], "bb");
        assert_eq!(e.text_lines()[1], "aa");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_to_end_keeps_head() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteToEnd);
        assert_eq!(e.text_lines()[0], "ab");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn delete_to_end_empty_is_noop() {
        let mut e = editor_with("");
        e.apply(Action::DeleteToEnd);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "");
    }

    #[test]
    fn change_line_clears_and_groups_typing() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeLine);
        assert_eq!(e.text_lines()[0], "");
        assert_eq!(e.mode(), Mode::Insert);
        e.apply(Action::InsertChar('x'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "abcdef");
    }

    #[test]
    fn change_to_end_positions_cursor() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::ChangeToEnd);
        assert_eq!(e.text_lines()[0], "ab");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn insert_before_selection_starts_at_start() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::InsertBeforeSelection);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.selection_range(), None);
    }

    #[test]
    fn insert_after_selection_starts_past_end() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::InsertAfterSelection);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 4));
        assert_eq!(e.selection_range(), None);
    }

    #[test]
    fn collapse_selection_keeps_point() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::CollapseSelection);
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 3), (0, 3))));
    }

    #[test]
    fn visual_yank_keeps_text() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::YankSelection);
        assert_eq!(e.text_lines()[0], "abcdef");
        assert_eq!((e.row(), e.col()), (0, 1));
        assert_eq!(e.mode(), Mode::Visual);
        assert_eq!(e.selection_range(), Some(((0, 1), (0, 3))));
        assert!(!e.is_dirty());
    }

    #[test]
    fn paste_after_deleted_text() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::EnterVisual);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteSelection);
        e.apply(Action::PasteAfter);
        assert_eq!(e.text_lines()[0], "aebcdf");
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn paste_empty_register_keeps_history() {
        let mut e = editor_with("ab");
        e.apply(Action::PasteAfter);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "ab");
    }

    #[test]
    fn word_forward_moves_to_start() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveToNextWordStart);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn word_backward_moves_to_start() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::MoveToPrevWordStart);
        assert_eq!((e.row(), e.col()), (0, 4));
        e.apply(Action::MoveToPrevWordStart);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn word_end_moves_to_end() {
        let mut e = editor_with("foo bar");
        e.apply(Action::MoveToNextWordEnd);
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn delete_word_deletes_forward() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWord);
        assert_eq!(e.text_lines()[0], "bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_last_word_trims() {
        let mut e = editor_with("foo bar");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteWord);
        assert_eq!(e.text_lines()[0], "foo ");
        assert_eq!((e.row(), e.col()), (0, 3));
    }

    #[test]
    fn delete_word_undoes_together() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWord);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "foo bar");
    }

    #[test]
    fn change_word_keeps_trailing_space() {
        let mut e = editor_with("foo bar");
        e.apply(Action::ChangeWord);
        assert_eq!(e.text_lines()[0], " bar");
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn change_word_groups_typing() {
        let mut e = editor_with("foo bar");
        e.apply(Action::ChangeWord);
        e.apply(Action::InsertChar('b'));
        e.apply(Action::InsertChar('a'));
        e.apply(Action::InsertChar('z'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "foo bar");
    }

    #[test]
    fn delete_word_backward_deletes_back() {
        let mut e = editor_with("foo bar");
        for _ in 0..4 {
            e.apply(Action::MoveRight);
        }
        e.apply(Action::DeleteWordBackward);
        assert_eq!(e.text_lines()[0], "bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_backward_joins_lines() {
        let mut e = editor_with("ab\ncd\nef");
        e.apply(Action::MoveDown);
        e.apply(Action::MoveDown);
        e.apply(Action::DeleteWordBackward);
        assert_eq!(e.text_lines().len(), 2);
        assert_eq!(e.text_lines()[0], "ab");
        assert_eq!(e.text_lines()[1], "ef");
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn delete_word_end_deletes_to_end() {
        let mut e = editor_with("foo bar");
        e.apply(Action::DeleteWordEnd);
        assert_eq!(e.text_lines()[0], " bar");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn delete_word_end_joins_lines() {
        let mut e = editor_with("foo\nbar");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::DeleteWordEnd);
        assert_eq!(e.text_lines().len(), 1);
        assert_eq!(e.text_lines()[0], "fo");
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn jump_clamps_to_last_char_in_normal() {
        let mut e = editor_with("abcdef");
        e.apply(Action::JumpTo(0, 20));
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn jump_reaches_end_in_insert() {
        let mut e = editor_with("abcdef");
        e.apply(Action::EnterInsert);
        e.apply(Action::JumpTo(0, 20));
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn jump_remembers_goal_column() {
        let mut e = editor_with("0123456789\nabcd");
        e.apply(Action::JumpTo(1, 2));
        e.apply(Action::MoveUp);
        assert_eq!((e.row(), e.col()), (0, 2));
    }

    #[test]
    fn append_steps_right_and_inserts() {
        let mut e = editor_with("ab");
        e.apply(Action::Append);
        assert_eq!(e.mode(), Mode::Insert);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn append_at_end_reaches_past_last() {
        let mut e = editor_with("ab");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::ExitInsert);
        e.apply(Action::Append);
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn append_on_empty_stays_zero() {
        let mut e = editor_with("");
        e.apply(Action::Append);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn append_line_end_reaches_end() {
        let mut e = editor_with("abcdef");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::AppendLineEnd);
        assert_eq!((e.row(), e.col()), (0, 6));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn insert_line_start_finds_content() {
        let mut e = editor_with("  abc");
        e.apply(Action::MoveToLineEnd);
        e.apply(Action::InsertLineStart);
        assert_eq!((e.row(), e.col()), (0, 2));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn insert_line_start_blank_goes_to_end() {
        let mut e = editor_with("   ");
        e.apply(Action::InsertLineStart);
        assert_eq!((e.row(), e.col()), (0, 3));
        assert_eq!(e.mode(), Mode::Insert);
    }

    #[test]
    fn append_line_end_groups_typing() {
        let mut e = editor_with("ab");
        e.apply(Action::AppendLineEnd);
        e.apply(Action::InsertChar('c'));
        e.apply(Action::ExitInsert);
        e.apply(Action::Undo);
        assert_eq!(e.text_lines()[0], "ab");
    }
}
