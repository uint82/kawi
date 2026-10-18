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
    FileExists,
    CannotOpen(String),
    Io(String),
    InvalidMark,
    NoString,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BufferError {
    Missing(usize),
    NotFound(String),
    Ambiguous(String),
    NoWrite(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteOutcome {
    pub lines: usize,
    pub bytes: usize,
    pub created: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MatchStage {
    Exact,
    Suffix,
    Prefix,
    Contains,
}

impl MatchStage {
    fn matches(self, name: &str, pattern: &str) -> bool {
        match self {
            Self::Exact => name == pattern,
            Self::Suffix => name.ends_with(pattern),
            Self::Prefix => name.starts_with(pattern),
            Self::Contains => name.contains(pattern),
        }
    }
}

pub struct Buffers {
    parked: Vec<BufferEntry>,
    current: usize,
    current_listed: bool,
    alternate: Option<usize>,
    next_number: usize,
}

struct BufferEntry {
    number: usize,
    state: ParkedState,
}

struct ParkedState {
    buffer: Buffer,
    listed: bool,
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
            current_listed: true,
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

    fn set_listed(&mut self, number: usize, listed: bool) {
        if number == self.current {
            self.current_listed = listed;
            return;
        }
        if let Some(entry) = self.parked.iter_mut().find(|entry| entry.number == number) {
            entry.state.listed = listed;
        }
    }

    fn entry(&self, number: usize) -> Option<&BufferEntry> {
        self.parked.iter().find(|entry| entry.number == number)
    }

    pub(super) fn line_count(&self, number: usize) -> Option<usize> {
        self.entry(number)
            .map(|entry| entry.state.buffer.line_count())
    }

    pub(super) fn numbers(&self) -> Vec<usize> {
        let mut numbers: Vec<usize> = self.parked.iter().map(|entry| entry.number).collect();
        numbers.push(self.current);
        numbers.sort_unstable();
        numbers
    }

    fn listed_numbers(&self) -> Vec<usize> {
        let mut numbers: Vec<usize> = self
            .parked
            .iter()
            .filter(|entry| entry.state.listed)
            .map(|entry| entry.number)
            .collect();
        if self.current_listed {
            numbers.push(self.current);
        }
        numbers.sort_unstable();
        numbers
    }
}

impl Editor {
    pub fn save(&mut self) -> Result<WriteOutcome, EditError> {
        if !self.has_file() {
            return Err(EditError::NoName);
        }
        let path = self.buffer.path.clone().expect("file buffers carry a path");
        self.write_current(&path)
    }

    pub fn write_as(&mut self, name: &str, force: bool) -> Result<WriteOutcome, EditError> {
        let target = paths::absolute(name, &current_dir());
        let current = self
            .buffer
            .path
            .as_deref()
            .map(|path| paths::absolute(path, &current_dir()));
        if current.as_deref() == Some(target.as_path()) {
            return self.write_current(name);
        }
        if target.exists() && !force {
            return Err(EditError::FileExists);
        }
        let created = !target.exists();
        self.buffer
            .write_to(name)
            .map_err(|error| EditError::Io(error.to_string()))?;
        if current.is_none() {
            self.buffer.adopt_path(name);
        }
        Ok(self.write_outcome(created))
    }

    fn write_current(&mut self, name: &str) -> Result<WriteOutcome, EditError> {
        let created = !current_dir().join(name).exists();
        self.buffer
            .save()
            .map_err(|error| EditError::Io(error.to_string()))?;
        Ok(self.write_outcome(created))
    }

    fn write_outcome(&self, created: bool) -> WriteOutcome {
        WriteOutcome {
            lines: self.buffer.line_count(),
            bytes: self.buffer.file_bytes(),
            created,
        }
    }

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
                    Some(number) if number == self.buffers.current() => {
                        self.buffers.set_listed(number, true);
                        self.reload_current(force)
                    }
                    Some(number) => {
                        self.buffers.set_listed(number, true);
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

    pub fn buffer_select_pattern(&mut self, pattern: &str) -> Result<(), BufferError> {
        if pattern == "%" {
            return Ok(());
        }
        let number = self.match_buffer(pattern)?;
        self.switch_buffer(number);
        Ok(())
    }

    pub fn buffer_delete(&mut self, spec: Option<&str>, force: bool) -> Result<(), BufferError> {
        let target = match spec {
            None => self.buffers.current(),
            Some(spec) => self.match_buffer(spec)?,
        };
        let modified = if target == self.buffers.current() {
            self.is_dirty()
        } else {
            self.buffers
                .entry(target)
                .is_some_and(|entry| entry.state.buffer.is_dirty())
        };
        if modified && !force {
            return Err(BufferError::NoWrite(target));
        }
        let previous = self.buffers.current();
        self.buffers.set_listed(target, false);
        if target == previous {
            match self.delete_replacement(target) {
                Some(next) => self.switch_buffer(next),
                None => {
                    self.open_empty_buffer();
                    self.buffers.set_alternate(Some(previous));
                }
            }
        }
        Ok(())
    }

    fn delete_replacement(&self, target: usize) -> Option<usize> {
        let alternate = self.buffers.alternate();
        if let Some(number) = alternate {
            if number != target && self.buffers.listed_numbers().contains(&number) {
                return Some(number);
            }
        }
        self.buffers
            .listed_numbers()
            .into_iter()
            .find(|number| *number != target)
    }

    fn match_buffer(&self, pattern: &str) -> Result<usize, BufferError> {
        let listed = self.buffers.listed_numbers();
        for pass in [&listed, &self.buffers.numbers()] {
            match self.match_within(pass, pattern) {
                Err(BufferError::NotFound(_)) => {}
                outcome => return outcome,
            }
        }
        Err(BufferError::NotFound(pattern.to_string()))
    }

    fn match_within(&self, numbers: &[usize], pattern: &str) -> Result<usize, BufferError> {
        let mut ambiguous = false;
        for stage in [
            MatchStage::Contains,
            MatchStage::Prefix,
            MatchStage::Suffix,
            MatchStage::Exact,
        ] {
            let found: Vec<usize> = numbers
                .iter()
                .copied()
                .filter(|number| self.buffer_matches(*number, stage, pattern))
                .collect();
            match found.len() {
                1 => return Ok(found[0]),
                0 => {}
                _ => ambiguous = true,
            }
        }
        if ambiguous {
            Err(BufferError::Ambiguous(pattern.to_string()))
        } else {
            Err(BufferError::NotFound(pattern.to_string()))
        }
    }

    fn buffer_matches(&self, number: usize, stage: MatchStage, pattern: &str) -> bool {
        self.buffer_names(number)
            .iter()
            .any(|name| stage.matches(name, pattern))
    }

    fn buffer_names(&self, number: usize) -> Vec<String> {
        let path = if number == self.buffers.current() {
            self.buffer.path.clone()
        } else {
            self.buffers
                .entry(number)
                .and_then(|entry| entry.state.buffer.path.clone())
        };
        let Some(path) = path else {
            return Vec::new();
        };
        let absolute = paths::absolute(&path, &current_dir())
            .to_string_lossy()
            .into_owned();
        let mut names = vec![path.clone(), absolute.clone()];
        for name in [&path, &absolute] {
            let home = home_path(name);
            if home != *name {
                names.push(home);
            }
        }
        names
    }

    pub fn buffer_number(&self) -> usize {
        self.buffers.current()
    }

    pub fn buffer_count(&self) -> usize {
        self.buffers.parked.len().saturating_add(1)
    }

    pub fn listed_buffer_count(&self) -> usize {
        self.buffers.listed_numbers().len()
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
            .filter(|entry| entry.state.listed)
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
            .collect();
        if self.buffers.current_listed {
            rows.push((
                self.buffers.current(),
                self.buffer_row(
                    self.buffers.current(),
                    self.buffer.path.as_deref(),
                    self.is_dirty(),
                    self.cursor.row().saturating_add(1),
                ),
            ));
        }
        rows.sort_by_key(|(number, _)| *number);
        rows.into_iter()
            .map(|(_, row)| row)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn write_all(&mut self) -> Result<(), EditError> {
        if self.is_dirty() {
            if !self.has_file() {
                return Err(EditError::NoName);
            }
            self.save()?;
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
        let name = match path {
            Some(path) => home_path(path),
            None => "[No Name]".to_string(),
        };
        let row = format!("{number:>3} {marker}{visible} {changed} \"{name}\"");
        let padding = 40usize.saturating_sub(row.chars().count()).max(1);
        format!("{row}{}line {line}", " ".repeat(padding))
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

    fn open_empty_buffer(&mut self) {
        self.store_current();
        let number = self.buffers.allocate_number();
        self.load_fresh(Buffer::from_string(""));
        self.buffers.set_current(number);
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
        let numbers = self.buffers.listed_numbers();
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
            listed: self.buffers.current_listed,
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
        self.buffers.current_listed = state.listed;
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
        self.buffers.current_listed = true;
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
        self.pending_autoindent = None;
        self.command.clear();
    }
}

fn home_path(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    home_shortened(&home, path)
}

fn home_shortened(home: &str, path: &str) -> String {
    if home.is_empty() {
        return path.to_string();
    }
    match path.strip_prefix(home) {
        Some(rest) if rest.starts_with('/') => format!("~{rest}"),
        _ => path.to_string(),
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
                "  1 #h + \"{}\" line 1\n  2 %a   \"{}\" line 1",
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
    fn write_as_names_an_unnamed_buffer() {
        let dir = temp_dir("write_name");
        let mut e = Editor::new(Buffer::from_string("alpha"));
        let target = file_path(&dir, "named.txt");
        let outcome = e.write_as(&target, false).unwrap();
        assert_eq!(
            outcome,
            WriteOutcome {
                lines: 1,
                bytes: 6,
                created: true
            }
        );
        assert_eq!(e.filename(), target);
        assert!(!e.is_dirty());
        assert_eq!(read_file(&dir, "named.txt"), "alpha\n");
    }

    #[test]
    fn write_as_copies_without_renaming() {
        let dir = temp_dir("write_copy");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        type_text(&mut e, "x");
        let target = file_path(&dir, "copy.txt");
        let outcome = e.write_as(&target, false).unwrap();
        assert!(outcome.created);
        assert_eq!(read_file(&dir, "copy.txt"), "xalpha\n");
        assert_eq!(e.filename(), file_path(&dir, "a.txt"));
        assert!(e.is_dirty());
    }

    #[test]
    fn write_as_refuses_existing_target_without_force() {
        let dir = temp_dir("write_exists");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "exists.txt", "old\n");
        let mut e = editor_in(&dir, "a.txt");
        let target = file_path(&dir, "exists.txt");
        assert_eq!(e.write_as(&target, false), Err(EditError::FileExists));
        e.write_as(&target, true).unwrap();
        assert_eq!(read_file(&dir, "exists.txt"), "alpha\n");
    }

    #[test]
    fn write_as_own_file_saves_and_cleans() {
        let dir = temp_dir("write_own");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        type_text(&mut e, "x");
        let outcome = e.write_as(&file_path(&dir, "a.txt"), false).unwrap();
        assert!(!outcome.created);
        assert!(!e.is_dirty());
    }

    #[test]
    fn delete_buffer_unlists_parked_buffer() {
        let dir = temp_dir("delete_parked");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        write_file(&dir, "c.txt", "charlie\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        open(&mut e, &dir, "c.txt");
        e.buffer_delete(Some(&file_path(&dir, "a.txt")), false)
            .unwrap();
        assert_eq!(e.listed_buffer_count(), 2);
        assert_eq!(e.buffer_count(), 3);
        assert_eq!(e.filename(), file_path(&dir, "c.txt"));
    }

    #[test]
    fn delete_current_switches_to_alternate() {
        let dir = temp_dir("delete_current");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        e.buffer_delete(None, false).unwrap();
        assert_eq!(e.listed_buffer_count(), 1);
        assert_eq!(e.filename(), file_path(&dir, "a.txt"));
    }

    #[test]
    fn delete_modified_buffer_requires_force() {
        let dir = temp_dir("delete_modified");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        type_text(&mut e, "x");
        assert_eq!(e.buffer_delete(None, false), Err(BufferError::NoWrite(1)));
        assert_eq!(e.listed_buffer_count(), 1);
        e.buffer_delete(None, true).unwrap();
        assert_eq!(e.listed_buffer_count(), 1);
        assert_eq!(e.filename(), "[No Name]");
    }

    #[test]
    fn delete_last_buffer_opens_empty_buffer() {
        let dir = temp_dir("delete_last");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        e.buffer_delete(None, false).unwrap();
        assert_eq!(e.listed_buffer_count(), 1);
        assert_eq!(e.buffer_number(), 2);
        assert_eq!(e.filename(), "[No Name]");
        assert!(e.is_empty());
    }

    #[test]
    fn match_buffer_prefers_tighter_matches() {
        let mut e = Editor::new(Buffer::from_string(""));
        for name in ["ab.txt", "b.txt", "abc.txt", "xb.txt"] {
            e.edit_file(Some(name), false).unwrap();
        }
        assert_eq!(e.match_buffer("b.txt").unwrap(), 2, "exact name");
        assert_eq!(e.match_buffer("b").unwrap(), 2, "prefix beats contains");
        assert_eq!(e.match_buffer("xb").unwrap(), 4, "contains");
        assert_eq!(
            e.match_buffer("ab").unwrap_err(),
            BufferError::Ambiguous("ab".to_string())
        );
        assert_eq!(
            e.match_buffer("zzz").unwrap_err(),
            BufferError::NotFound("zzz".to_string())
        );
    }

    #[test]
    fn home_shortening_only_matches_directory_prefixes() {
        assert_eq!(home_shortened("/home/user", "/home/user/x"), "~/x");
        assert_eq!(home_shortened("/home/user", "/home/user"), "/home/user");
        assert_eq!(
            home_shortened("/home/user", "/home/userx/x"),
            "/home/userx/x"
        );
        assert_eq!(home_shortened("", "/home/user/x"), "/home/user/x");
        assert_eq!(home_shortened("/home/user", "x.txt"), "x.txt");
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
