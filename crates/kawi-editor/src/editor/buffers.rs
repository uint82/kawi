use std::path::{Path, PathBuf};

use kawi_buffer::Buffer;

use super::indent::{self, IndentConfig};
use super::repeat::RepeatChange;
use super::session::InsertEdit;
use super::SearchStatus;
use crate::cursor::Cursor;
use crate::mode::Mode;
use crate::paths;

use super::Editor;

fn current_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    NoName,
    NoWrite,
    CannotOpen(String),
    Io(String),
    InvalidMark,
}

pub struct Buffers {
    parked: Vec<BufferEntry>,
    current: usize,
    alternate: Option<usize>,
    next_number: usize,
}

struct BufferEntry {
    number: usize,
    state: ParkedState,
}

struct ParkedState {
    buffer: Buffer,
    cursor: Cursor,
    file_indent: IndentConfig,
    viewport_top: usize,
    last_change: Option<RepeatChange>,
    undone_change: Option<RepeatChange>,
    insert_repeat: Option<usize>,
    insert_reopen: bool,
    insert_record: Vec<InsertEdit>,
    insert_moved: bool,
    insert_last_char: Option<char>,
    search_status: Option<SearchStatus>,
}

impl Buffers {
    pub(super) fn new() -> Self {
        Self {
            parked: Vec::new(),
            current: 1,
            alternate: None,
            next_number: 2,
        }
    }

    pub(super) fn current(&self) -> usize {
        self.current
    }

    pub(super) fn set_current(&mut self, number: usize) {
        self.current = number;
    }

    pub(super) fn alternate(&self) -> Option<usize> {
        self.alternate
    }

    pub(super) fn set_alternate(&mut self, number: Option<usize>) {
        self.alternate = number;
    }

    fn allocate_number(&mut self) -> usize {
        let number = self.next_number;
        self.next_number = self.next_number.saturating_add(1);
        number
    }

    pub(super) fn contains(&self, number: usize) -> bool {
        number == self.current || self.parked.iter().any(|entry| entry.number == number)
    }

    fn take(&mut self, number: usize) -> Option<ParkedState> {
        let index = self
            .parked
            .iter()
            .position(|entry| entry.number == number)?;
        Some(self.parked.remove(index).state)
    }

    fn insert(&mut self, number: usize, state: ParkedState) {
        self.parked.push(BufferEntry { number, state });
    }

    fn entry(&self, number: usize) -> Option<&BufferEntry> {
        self.parked.iter().find(|entry| entry.number == number)
    }

    pub(super) fn line_count(&self, number: usize) -> Option<usize> {
        self.entry(number)
            .map(|entry| entry.state.buffer.line_count())
    }

    fn numbers(&mut self) -> Vec<usize> {
        let mut numbers: Vec<usize> = self.parked.iter().map(|entry| entry.number).collect();
        numbers.push(self.current);
        numbers.sort_unstable();
        numbers
    }
}

impl Editor {
    pub fn has_file(&self) -> bool {
        self.buffer.path.is_some()
    }

    pub fn has_unsaved(&self) -> bool {
        self.is_dirty()
            || self
                .buffers
                .parked
                .iter()
                .any(|entry| entry.state.buffer.is_dirty())
    }

    pub fn edit_file(&mut self, name: Option<&str>, force: bool) -> Result<(), EditError> {
        match name {
            None => self.reload_current(force),
            Some(name) => {
                let target = paths::absolute(name, &current_dir());
                match self.find_buffer(&target) {
                    Some(number) if number == self.buffers.current() => self.reload_current(force),
                    Some(number) => {
                        self.switch_buffer(number);
                        Ok(())
                    }
                    None => self.open_buffer(name),
                }
            }
        }
    }

    pub fn buffer_next(&mut self) {
        self.step_buffer(1);
    }

    pub fn buffer_previous(&mut self) {
        self.step_buffer(-1);
    }

    pub fn buffer_alternate(&mut self) {
        if let Some(number) = self.buffers.alternate() {
            self.switch_buffer(number);
        }
    }

    pub fn buffer_select(&mut self, number: usize) -> bool {
        if !self.buffers.contains(number) {
            return false;
        }
        self.switch_buffer(number);
        true
    }

    pub fn buffer_number(&self) -> usize {
        self.buffers.current()
    }

    pub fn buffer_count(&self) -> usize {
        self.buffers.parked.len().saturating_add(1)
    }

    pub fn alternate_buffer(&self) -> Option<usize> {
        self.buffers.alternate()
    }

    pub fn buffer_name(&self, number: usize) -> Option<&str> {
        if number == self.buffers.current() {
            return Some(self.filename());
        }
        self.buffers
            .entry(number)
            .map(|entry| entry.state.buffer.path.as_deref().unwrap_or("[No Name]"))
    }

    pub fn buffer_list(&self) -> String {
        let mut rows: Vec<(usize, String)> = self
            .buffers
            .parked
            .iter()
            .map(|entry| {
                let line = entry.state.cursor.row().saturating_add(1);
                (
                    entry.number,
                    self.buffer_row(
                        entry.number,
                        entry.state.buffer.path.as_deref(),
                        entry.state.buffer.is_dirty(),
                        line,
                    ),
                )
            })
            .chain(std::iter::once((
                self.buffers.current(),
                self.buffer_row(
                    self.buffers.current(),
                    self.buffer.path.as_deref(),
                    self.is_dirty(),
                    self.cursor.row().saturating_add(1),
                ),
            )))
            .collect();
        rows.sort_by_key(|(number, _)| *number);
        rows.into_iter()
            .map(|(_, row)| row)
            .collect::<Vec<_>>()
            .join(" | ")
    }

    pub fn write_all(&mut self) -> Result<(), EditError> {
        if self.is_dirty() {
            if !self.has_file() {
                return Err(EditError::NoName);
            }
            self.save()
                .map_err(|error| EditError::Io(error.to_string()))?;
        }
        for entry in self.buffers.parked.iter_mut() {
            if !entry.state.buffer.is_dirty() {
                continue;
            }
            if !entry.state.buffer.has_path() {
                return Err(EditError::NoName);
            }
            entry
                .state
                .buffer
                .save()
                .map_err(|error| EditError::Io(error.to_string()))?;
        }
        Ok(())
    }

    fn buffer_row(&self, number: usize, path: Option<&str>, dirty: bool, line: usize) -> String {
        let marker = if number == self.buffers.current() {
            '%'
        } else if self.buffers.alternate() == Some(number) {
            '#'
        } else {
            ' '
        };
        let visible = if number == self.buffers.current() {
            'a'
        } else {
            'h'
        };
        let changed = if dirty { '+' } else { ' ' };
        let name = path.unwrap_or("[No Name]");
        format!("{number:>3} {marker}{visible} {changed} \"{name}\" line {line}")
    }

    fn find_buffer(&self, target: &Path) -> Option<usize> {
        if self
            .buffer
            .path
            .as_deref()
            .is_some_and(|path| paths::absolute(path, &current_dir()) == target)
        {
            return Some(self.buffers.current());
        }
        self.buffers.parked.iter().find_map(|entry| {
            let path = entry.state.buffer.path.as_deref()?;
            (paths::absolute(path, &current_dir()) == target).then_some(entry.number)
        })
    }

    fn reload_current(&mut self, force: bool) -> Result<(), EditError> {
        if !self.has_file() {
            return Err(EditError::NoName);
        }
        if self.is_dirty() && !force {
            return Err(EditError::NoWrite);
        }
        let (row, col) = (self.cursor.row(), self.cursor.col());
        self.buffer.checkpoint(row, col);
        self.buffer
            .reload_from_disk()
            .map_err(|error| EditError::Io(error.to_string()))?;
        self.buffer.mark_clean();
        self.normalize_cursor();
        self.remember_wanted_column();
        self.reveal_cursor();
        Ok(())
    }

    fn open_buffer(&mut self, name: &str) -> Result<(), EditError> {
        if self.buffer_reusable() {
            return self.reuse_buffer(name);
        }
        let buffer = Buffer::open(name).map_err(|_| EditError::CannotOpen(name.to_string()))?;
        let jump = self.current_jump();
        self.jumps.record(jump);
        let previous = self.buffers.current();
        self.store_current();
        let number = self.buffers.allocate_number();
        self.load_fresh(buffer);
        self.buffers.set_current(number);
        self.buffers.set_alternate(Some(previous));
        Ok(())
    }

    fn buffer_reusable(&self) -> bool {
        !self.has_file() && self.buffer.is_empty() && !self.is_dirty()
    }

    fn reuse_buffer(&mut self, name: &str) -> Result<(), EditError> {
        let buffer = Buffer::open(name).map_err(|_| EditError::CannotOpen(name.to_string()))?;
        let number = self.buffers.current();
        self.jumps.record(self.current_jump());
        self.load_fresh(buffer);
        self.buffers.set_alternate(Some(number));
        Ok(())
    }

    fn step_buffer(&mut self, direction: isize) {
        let numbers = self.buffers.numbers();
        if numbers.len() < 2 {
            return;
        }
        let position = numbers
            .iter()
            .position(|number| *number == self.buffers.current())
            .unwrap_or(0);
        let next = (position as isize + direction).rem_euclid(numbers.len() as isize) as usize;
        self.switch_buffer(numbers[next]);
    }

    pub(super) fn switch_buffer(&mut self, number: usize) {
        if number == self.buffers.current() || !self.buffers.contains(number) {
            return;
        }
        let previous = self.buffers.current();
        self.store_current();
        if let Some(state) = self.buffers.take(number) {
            self.restore_state(state);
        }
        self.buffers.set_current(number);
        self.buffers.set_alternate(Some(previous));
        self.remember_wanted_column();
        self.reveal_cursor();
    }

    pub(super) fn current_jump(&self) -> crate::jumps::Jump {
        crate::jumps::Jump {
            buffer: self.buffers.current(),
            row: self.cursor.row(),
            col: self.cursor.col(),
        }
    }

    fn store_current(&mut self) {
        let state = self.take_state();
        self.buffers.insert(self.buffers.current(), state);
    }

    fn take_state(&mut self) -> ParkedState {
        ParkedState {
            buffer: std::mem::replace(&mut self.buffer, Buffer::from_string("")),
            cursor: std::mem::take(&mut self.cursor),
            file_indent: std::mem::replace(&mut self.file_indent, indent::detect(None)),
            viewport_top: self.viewport.top(),
            last_change: self.last_change.take(),
            undone_change: self.undone_change.take(),
            insert_repeat: self.insert_repeat.take(),
            insert_reopen: std::mem::take(&mut self.insert_reopen),
            insert_record: std::mem::take(&mut self.insert_record),
            insert_moved: std::mem::take(&mut self.insert_moved),
            insert_last_char: self.insert_last_char.take(),
            search_status: self.search_status.take(),
        }
    }

    fn restore_state(&mut self, state: ParkedState) {
        self.buffer = state.buffer;
        self.cursor = state.cursor;
        self.file_indent = state.file_indent;
        self.viewport
            .place_top(state.viewport_top, self.buffer.line_count());
        self.last_change = state.last_change;
        self.undone_change = state.undone_change;
        self.insert_repeat = state.insert_repeat;
        self.insert_reopen = state.insert_reopen;
        self.insert_record = state.insert_record;
        self.insert_moved = state.insert_moved;
        self.insert_last_char = state.insert_last_char;
        self.search_status = state.search_status;
        self.reset_transient();
    }

    fn load_fresh(&mut self, buffer: Buffer) {
        self.file_indent = indent::detect(buffer.path.as_deref());
        self.buffer = buffer;
        self.cursor = Cursor::default();
        self.viewport.place_top(0, self.buffer.line_count());
        self.last_change = None;
        self.undone_change = None;
        self.insert_repeat = None;
        self.insert_reopen = false;
        self.insert_record.clear();
        self.insert_moved = false;
        self.insert_last_char = None;
        self.search_status = None;
        self.reset_transient();
        self.remember_wanted_column();
    }

    fn reset_transient(&mut self) {
        self.mode = Mode::Normal;
        self.selection = None;
        self.previous_visual = None;
        self.last_insert = None;
        self.insert_entry = None;
        self.pending_register = None;
        self.force_numbered_register = false;
        self.clear_count();
        self.search_origin = None;
        self.command.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use std::fs;
    use std::path::PathBuf;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kawi_buffers_{name}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn file_path(dir: &Path, name: &str) -> String {
        dir.join(name).to_string_lossy().into_owned()
    }

    fn write_file(dir: &Path, name: &str, text: &str) {
        fs::write(dir.join(name), text).unwrap();
    }

    fn read_file(dir: &Path, name: &str) -> String {
        fs::read_to_string(dir.join(name)).unwrap()
    }

    fn editor_in(dir: &Path, name: &str) -> Editor {
        Editor::new(Buffer::from_file(&file_path(dir, name)).unwrap())
    }

    fn open(editor: &mut Editor, dir: &Path, name: &str) {
        editor
            .edit_file(Some(&file_path(dir, name)), false)
            .unwrap();
    }

    fn type_text(editor: &mut Editor, text: &str) {
        editor.apply(Action::EnterInsert);
        for ch in text.chars() {
            editor.apply(Action::InsertChar(ch));
        }
        editor.apply(Action::ExitInsert);
    }

    fn lines(editor: &Editor) -> Vec<String> {
        editor.buffer.window_lines(0, editor.buffer.line_count())
    }

    #[test]
    fn edit_file_opens_a_new_buffer() {
        let dir = temp_dir("open");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\ncharlie\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        assert_eq!(e.filename(), file_path(&dir, "b.txt"));
        assert_eq!(lines(&e), vec!["bravo", "charlie"]);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.buffers.current(), 2);
        assert_eq!(e.buffers.alternate(), Some(1));
    }

    #[test]
    fn edit_file_reuses_an_open_buffer() {
        let dir = temp_dir("reuse");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        let dotted = format!("{}/./b.txt", dir.display());
        e.edit_file(Some(&dotted), false).unwrap();
        assert_eq!(e.buffers.current(), 2);
        open(&mut e, &dir, "a.txt");
        assert_eq!(e.buffers.current(), 1);
        assert_eq!(e.buffers.alternate(), Some(2));
    }

    #[test]
    fn edit_file_normalizes_dot_segments_for_identity() {
        let dir = temp_dir("normalize");
        fs::create_dir_all(dir.join("sub")).unwrap();
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        let dotted = format!("{}/sub/../a.txt", dir.display());
        e.edit_file(Some(&dotted), false).unwrap();
        assert_eq!(e.buffers.current(), 1);
    }

    #[test]
    fn edit_unnamed_buffer_requires_a_name() {
        let mut e = Editor::new(Buffer::from_string(""));
        assert_eq!(e.edit_file(None, false), Err(EditError::NoName));
    }

    #[test]
    fn edit_file_reuses_an_empty_unnamed_buffer() {
        let dir = temp_dir("reuse_unnamed");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = Editor::new(Buffer::from_string(""));
        e.edit_file(Some(&file_path(&dir, "a.txt")), false).unwrap();
        assert_eq!(e.buffer_number(), 1);
        assert_eq!(e.buffer_count(), 1);
        assert_eq!(e.filename(), file_path(&dir, "a.txt"));
        assert_eq!(e.alternate_buffer(), Some(1));
        assert_eq!(lines(&e), vec!["alpha"]);
    }

    #[test]
    fn edit_file_keeps_a_modified_unnamed_buffer() {
        let dir = temp_dir("reuse_modified");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = Editor::new(Buffer::from_string(""));
        type_text(&mut e, "x");
        e.edit_file(Some(&file_path(&dir, "a.txt")), false).unwrap();
        assert_eq!(e.buffer_number(), 2);
        assert_eq!(e.buffer_count(), 2);
        assert_eq!(e.alternate_buffer(), Some(1));
    }

    #[test]
    fn edit_reload_requires_force_when_modified() {
        let dir = temp_dir("reload_guard");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        type_text(&mut e, "x");
        assert_eq!(e.edit_file(None, false), Err(EditError::NoWrite));
        assert_eq!(lines(&e), vec!["xalpha"]);
        e.edit_file(None, true).unwrap();
        assert_eq!(lines(&e), vec!["alpha"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn edit_reload_refreshes_content_from_disk() {
        let dir = temp_dir("reload_disk");
        write_file(&dir, "a.txt", "alpha\nbravo\ncharlie\n");
        let mut e = editor_in(&dir, "a.txt");
        e.apply(Action::JumpTo(2, 1));
        write_file(&dir, "a.txt", "changed");
        e.edit_file(None, false).unwrap();
        assert_eq!(lines(&e), vec!["changed"]);
        assert_eq!((e.row(), e.col()), (0, 1));
    }

    #[test]
    fn edit_reload_can_be_undone() {
        let dir = temp_dir("reload_undo");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        write_file(&dir, "a.txt", "changed\n");
        e.edit_file(None, false).unwrap();
        e.apply(Action::Undo);
        assert_eq!(lines(&e), vec!["alpha"]);
        assert!(e.is_dirty());
    }

    #[test]
    fn edit_new_file_creates_empty_buffer_with_name() {
        let dir = temp_dir("new");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "fresh.txt");
        assert_eq!(e.filename(), file_path(&dir, "fresh.txt"));
        assert!(e.is_empty());
        assert!(!e.is_dirty());
    }

    #[test]
    fn switching_buffers_preserves_cursor_and_changes() {
        let dir = temp_dir("preserve");
        write_file(&dir, "a.txt", "one\ntwo\nthree\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        e.apply(Action::JumpTo(2, 1));
        open(&mut e, &dir, "b.txt");
        type_text(&mut e, "x");
        open(&mut e, &dir, "a.txt");
        assert_eq!((e.row(), e.col()), (2, 1));
        assert!(!e.is_dirty());
        open(&mut e, &dir, "b.txt");
        assert_eq!(lines(&e), vec!["xbravo"]);
        assert!(e.is_dirty());
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn undo_history_is_per_buffer() {
        let dir = temp_dir("undo_each");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        type_text(&mut e, "x");
        open(&mut e, &dir, "b.txt");
        type_text(&mut e, "y");
        open(&mut e, &dir, "a.txt");
        e.apply(Action::Undo);
        assert_eq!(lines(&e), vec!["alpha"]);
        open(&mut e, &dir, "b.txt");
        assert_eq!(lines(&e), vec!["ybravo"]);
    }

    #[test]
    fn next_and_previous_cycle_through_buffers() {
        let dir = temp_dir("cycle");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        assert_eq!(e.buffers.current(), 2);
        e.buffer_next();
        assert_eq!(e.buffers.current(), 1);
        e.buffer_next();
        assert_eq!(e.buffers.current(), 2);
        e.buffer_previous();
        assert_eq!(e.buffers.current(), 1);
    }

    #[test]
    fn next_with_single_buffer_is_noop() {
        let dir = temp_dir("single");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        e.buffer_next();
        e.buffer_previous();
        assert_eq!(e.buffers.current(), 1);
    }

    #[test]
    fn alternate_toggles_between_buffers() {
        let dir = temp_dir("alternate");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        e.buffer_alternate();
        assert_eq!(e.filename(), file_path(&dir, "a.txt"));
        e.buffer_alternate();
        assert_eq!(e.filename(), file_path(&dir, "b.txt"));
    }

    #[test]
    fn buffer_select_checks_existence() {
        let dir = temp_dir("select");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        assert!(e.buffer_select(1));
        assert_eq!(e.filename(), file_path(&dir, "a.txt"));
        assert!(!e.buffer_select(9));
        assert_eq!(e.filename(), file_path(&dir, "a.txt"));
    }

    #[test]
    fn buffer_list_shows_flags_and_names() {
        let dir = temp_dir("list");
        write_file(&dir, "a.txt", "one\ntwo\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        type_text(&mut e, "x");
        open(&mut e, &dir, "b.txt");
        assert_eq!(
            e.buffer_list(),
            format!(
                "  1 #h + \"{}\" line 1 |   2 %a   \"{}\" line 1",
                file_path(&dir, "a.txt"),
                file_path(&dir, "b.txt")
            )
        );
    }

    #[test]
    fn write_all_saves_every_modified_buffer() {
        let dir = temp_dir("write_all");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        type_text(&mut e, "x");
        open(&mut e, &dir, "b.txt");
        type_text(&mut e, "y");
        e.write_all().unwrap();
        assert!(!e.has_unsaved());
        assert_eq!(read_file(&dir, "a.txt"), "xalpha\n");
        assert_eq!(read_file(&dir, "b.txt"), "ybravo\n");
    }

    #[test]
    fn write_all_reports_missing_name() {
        let mut e = Editor::new(Buffer::from_string(""));
        type_text(&mut e, "x");
        assert_eq!(e.write_all(), Err(EditError::NoName));
    }

    #[test]
    fn search_pattern_survives_buffer_switches() {
        let dir = temp_dir("search_survives");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        e.apply(Action::EnterSearch);
        for ch in "alp".chars() {
            e.apply(Action::PushSearchChar(ch));
        }
        e.apply(Action::ConfirmSearch);
        open(&mut e, &dir, "b.txt");
        assert_eq!(e.search_pattern(), "alp");
    }

    #[test]
    fn edit_records_a_jump_and_navigates_back() {
        let dir = temp_dir("jump");
        write_file(&dir, "a.txt", "one\ntwo\nthree\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        e.apply(Action::JumpTo(2, 1));
        open(&mut e, &dir, "b.txt");
        e.jump_older().unwrap();
        assert_eq!(e.filename(), file_path(&dir, "a.txt"));
        assert_eq!((e.row(), e.col()), (2, 1));
    }

    #[test]
    fn jump_into_invalid_mark_reports_error() {
        let dir = temp_dir("jump_invalid");
        write_file(&dir, "a.txt", "one\ntwo\nthree\nfour\n");
        let mut e = editor_in(&dir, "a.txt");
        e.apply(Action::JumpTo(3, 0));
        e.apply(Action::GotoFirstLine);
        write_file(&dir, "a.txt", "one\n");
        e.edit_file(None, false).unwrap();
        assert_eq!(e.jump_older(), Err(EditError::InvalidMark));
    }
}
