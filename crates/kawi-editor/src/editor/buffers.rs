use std::path::{Path, PathBuf};

use kawi_buffer::{apply_text_edits, content_hash, Buffer, TextEdit};
use ropey::Rope;

use super::indent::{self, IndentConfig};
use super::repeat::RepeatChange;
use super::session::InsertEdit;
use super::SearchStatus;
use crate::cursor::Cursor;
use crate::mode::Mode;
use crate::paths;

use super::Editor;

pub(super) fn current_dir() -> PathBuf {
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
    MarkNotSet,
    NoString,
    InvalidEdit,
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
    runtime_shiftwidth: Option<usize>,
    runtime_expandtab: Option<bool>,
    language_indent: super::options::LanguageIndent,
    autoindent: bool,
    viewport_top: usize,
    last_change: Option<RepeatChange>,
    undone_change: Option<RepeatChange>,
    insert_repeat: Option<usize>,
    insert_reopen: bool,
    insert_record: Vec<InsertEdit>,
    insert_moved: bool,
    insert_last_char: Option<char>,
    search_status: Option<SearchStatus>,
    change_list: Vec<(usize, usize)>,
    change_index: usize,
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
    pub fn save(&mut self, force: bool) -> Result<WriteOutcome, EditError> {
        if !self.has_file() {
            return Err(EditError::NoName);
        }
        let path = self.buffer.path.clone().expect("file buffers carry a path");
        self.write_current(&path, force)
    }

    pub fn write_as(&mut self, name: &str, force: bool) -> Result<WriteOutcome, EditError> {
        let target = paths::absolute(name, &current_dir());
        let current = self
            .buffer
            .path
            .as_deref()
            .map(|path| paths::absolute(path, &current_dir()));
        if current.as_deref() == Some(target.as_path()) {
            return self.write_current(name, force);
        }
        if target.exists() && !force {
            return Err(EditError::FileExists);
        }
        let created = !target.exists();
        self.buffer
            .write_to(name, force)
            .map_err(|error| EditError::Io(error.to_string()))?;
        if current.is_none() {
            self.buffer.adopt_path(name);
        }
        Ok(self.write_outcome(created))
    }

    fn write_current(&mut self, name: &str, force: bool) -> Result<WriteOutcome, EditError> {
        let created = !current_dir().join(name).exists();
        self.buffer
            .save(force)
            .map_err(|error| EditError::Io(error.to_string()))?;
        Ok(self.write_outcome(created))
    }

    fn write_outcome(&self, created: bool) -> WriteOutcome {
        buffer_write_outcome(&self.buffer, created)
    }

    pub fn save_buffer(&mut self, number: usize, force: bool) -> Result<WriteOutcome, EditError> {
        if number == self.buffers.current() {
            return self.save(force);
        }
        let entry = self
            .buffers
            .parked
            .iter_mut()
            .find(|entry| entry.number == number)
            .ok_or(BufferError::Missing(number))
            .map_err(|_| EditError::NoName)?;
        let path = entry.state.buffer.path.clone().ok_or(EditError::NoName)?;
        let created = !paths::absolute(&path, &current_dir()).exists();
        entry
            .state
            .buffer
            .save(force)
            .map_err(|error| EditError::Io(error.to_string()))?;
        Ok(buffer_write_outcome(&entry.state.buffer, created))
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

    pub fn file_is_modified(&self, target: &Path) -> bool {
        match self.find_buffer(target) {
            Some(number) if number == self.buffers.current() => self.is_dirty(),
            Some(number) => self
                .buffers
                .entry(number)
                .is_some_and(|entry| entry.state.buffer.is_dirty()),
            None => false,
        }
    }

    pub fn reload_files(&mut self, targets: &[PathBuf]) {
        let mut targets = targets.to_vec();
        targets.sort();
        targets.dedup();
        for target in targets {
            self.reload_open_file(&target);
        }
    }

    fn reload_open_file(&mut self, target: &Path) {
        let Some(number) = self.find_buffer(target) else {
            return;
        };
        if self.file_is_modified(target) {
            return;
        }
        if number == self.buffers.current() {
            let _ = self.reload_current(true);
            return;
        }
        self.reload_parked_file(number);
    }

    fn reload_parked_file(&mut self, number: usize) {
        let _ = self.reload_parked_buffer(number);
    }

    fn reload_parked_buffer(&mut self, number: usize) -> Result<(), EditError> {
        let entry = self
            .buffers
            .parked
            .iter_mut()
            .find(|entry| entry.number == number)
            .ok_or(BufferError::Missing(number))
            .map_err(|_| EditError::NoName)?;
        let state = &mut entry.state;
        let row = state.cursor.row();
        let col = state.cursor.col();
        state.buffer.checkpoint(row, col);
        state
            .buffer
            .reload_from_disk()
            .map_err(|error| EditError::Io(error.to_string()))?;
        state.buffer.mark_clean();
        let row = row.min(state.buffer.line_count().saturating_sub(1));
        let col = col.min(state.buffer.line_len(row).saturating_sub(1));
        state.cursor.set_position(row, col);
        state.cursor.set_desired(col);
        Ok(())
    }

    pub fn reload_buffer(&mut self, number: usize) -> Result<(), EditError> {
        if number == self.buffers.current() {
            return self.reload_current(true);
        }
        self.reload_parked_buffer(number)
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
                    None => self.open_buffer(name, true),
                }
            }
        }
    }

    pub fn open_file_buffer(&mut self, name: &str) -> Result<(), EditError> {
        let target = paths::absolute(name, &current_dir());
        if let Some(number) = self.find_buffer(&target) {
            self.buffers.set_listed(number, true);
            if number != self.buffers.current() {
                self.switch_buffer(number);
            }
            return Ok(());
        }
        self.open_buffer(name, false)
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

    pub fn buffer_numbers(&self) -> Vec<usize> {
        self.buffers.listed_numbers()
    }

    pub fn all_buffer_numbers(&self) -> Vec<usize> {
        self.buffers.numbers()
    }

    pub fn buffer_id(&self, number: usize) -> Option<&str> {
        self.buffer_at(number).map(Buffer::id)
    }

    pub fn find_buffer_by_id(&self, id: &str) -> Option<usize> {
        self.buffers
            .numbers()
            .into_iter()
            .find(|number| self.buffer_id(*number) == Some(id))
    }

    pub fn set_buffer_id(&mut self, number: usize, id: String) -> bool {
        if number == self.buffers.current() {
            self.buffer.set_id(id);
            return true;
        }
        let Some(entry) = self
            .buffers
            .parked
            .iter_mut()
            .find(|entry| entry.number == number)
        else {
            return false;
        };
        entry.state.buffer.set_id(id);
        true
    }

    pub fn buffer_text(&self, number: usize) -> Option<String> {
        self.buffer_rope(number)
            .map(|snapshot| snapshot.to_string())
    }

    pub fn buffer_rope(&self, number: usize) -> Option<Rope> {
        self.buffer_at(number).map(Buffer::snapshot)
    }

    pub fn restore_buffer_text(&mut self, number: usize, text: &str) -> bool {
        if number == self.buffers.current() {
            self.buffer.checkpoint(self.cursor.row(), self.cursor.col());
            self.buffer.replace_all(text);
            return true;
        }
        let Some(entry) = self
            .buffers
            .parked
            .iter_mut()
            .find(|entry| entry.number == number)
        else {
            return false;
        };
        let row = entry.state.cursor.row();
        let col = entry.state.cursor.col();
        entry.state.buffer.checkpoint(row, col);
        entry.state.buffer.replace_all(text);
        true
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

    pub fn buffer_path(&self, number: usize) -> Option<&str> {
        self.buffer_at(number)
            .and_then(|buffer| buffer.path.as_deref())
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

    pub fn buffer_line_count(&self, number: usize) -> Option<usize> {
        self.buffer_line_count_at(number)
    }

    pub fn buffer_is_dirty(&self, number: usize) -> Option<bool> {
        self.buffer_is_dirty_at(number)
    }

    pub fn buffer_is_empty(&self, number: usize) -> Option<bool> {
        self.buffer_is_empty_at(number)
    }

    pub fn buffer_line_len(&self, number: usize, row: usize) -> Option<usize> {
        self.buffer_line_len_at(number, row)
    }

    pub fn buffer_virtcol(&self, number: usize, row: usize, col: usize) -> Option<usize> {
        let buffer = self.buffer_at(number)?;
        let tabstop = self.effective_indent().tabstop;
        let width = buffer.line_len(row);
        if col < width {
            Some(
                buffer
                    .screen_col(row, col.saturating_add(1), tabstop)
                    .saturating_sub(1),
            )
        } else {
            Some(buffer.screen_col(row, width, tabstop))
        }
    }

    pub fn buffer_window_lines(
        &self,
        number: usize,
        top: usize,
        height: usize,
    ) -> Option<Vec<String>> {
        self.buffer_at(number)
            .map(|buffer| buffer.window_lines(top, height))
    }

    pub fn buffer_revision_at(&self, number: usize) -> Option<u64> {
        self.buffer_at(number).map(Buffer::revision)
    }

    fn buffer_at(&self, number: usize) -> Option<&Buffer> {
        if number == self.buffers.current() {
            return Some(&self.buffer);
        }
        self.buffers.entry(number).map(|entry| &entry.state.buffer)
    }

    fn buffer_line_count_at(&self, number: usize) -> Option<usize> {
        self.buffer_at(number).map(Buffer::line_count)
    }

    pub fn buffer_tabstop(&self, number: usize) -> Option<usize> {
        if number == self.buffers.current() {
            return Some(self.effective_indent().tabstop);
        }
        self.buffers
            .entry(number)
            .map(|entry| entry.state.file_indent.tabstop)
    }

    fn buffer_is_dirty_at(&self, number: usize) -> Option<bool> {
        self.buffer_at(number).map(Buffer::is_dirty)
    }

    fn buffer_is_empty_at(&self, number: usize) -> Option<bool> {
        self.buffer_at(number).map(Buffer::is_empty)
    }

    fn buffer_line_len_at(&self, number: usize, row: usize) -> Option<usize> {
        self.buffer_at(number).map(|buffer| buffer.line_len(row))
    }

    pub fn discard_current_changes(&mut self) {
        if self.buffer.path.is_some() {
            let _ = self.buffer.reload_from_disk();
        } else {
            self.buffer.replace_all("");
        }
        self.buffer.mark_clean();
        self.clamp_row();
        self.clamp_col();
        self.remember_wanted_column();
        self.reveal_cursor();
    }

    pub fn write_all(&mut self) -> Result<(), EditError> {
        for number in self.buffers.numbers() {
            if self.buffer_at(number).is_some_and(Buffer::is_dirty) {
                self.save_buffer(number, false)?;
            }
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
            .is_some_and(|path| same_file_path(path, target))
        {
            return Some(self.buffers.current());
        }
        self.buffers.parked.iter().find_map(|entry| {
            let path = entry.state.buffer.path.as_deref()?;
            same_file_path(path, target).then_some(entry.number)
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

    pub fn open_empty_buffer(&mut self) {
        self.store_current();
        let number = self.buffers.allocate_number();
        self.load_fresh(Buffer::from_string(""));
        self.reset_buffer_options();
        self.buffers.set_current(number);
    }

    fn open_buffer(&mut self, name: &str, reuse: bool) -> Result<(), EditError> {
        if reuse && self.buffer_reusable() {
            return self.reuse_buffer(name);
        }
        let buffer = Buffer::open(name).map_err(|_| EditError::CannotOpen(name.to_string()))?;
        let jump = self.current_jump();
        self.jumps.record(jump);
        let previous = self.buffers.current();
        self.store_current();
        let number = self.buffers.allocate_number();
        self.load_fresh(buffer);
        self.reset_buffer_options();
        self.place_open_cursor();
        self.buffers.set_current(number);
        self.buffers.set_alternate(Some(previous));
        Ok(())
    }

    fn place_open_cursor(&mut self) {
        let col = self.first_non_blank_of(0);
        self.cursor.set_position(0, col);
        self.remember_wanted_column();
    }

    fn buffer_reusable(&self) -> bool {
        !self.has_file() && self.buffer.is_empty() && !self.is_dirty()
    }

    fn reuse_buffer(&mut self, name: &str) -> Result<(), EditError> {
        let buffer = Buffer::open(name).map_err(|_| EditError::CannotOpen(name.to_string()))?;
        let number = self.buffers.current();
        self.jumps.record(self.current_jump());
        self.load_fresh(buffer);
        self.place_open_cursor();
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
            runtime_shiftwidth: self.runtime_shiftwidth.take(),
            runtime_expandtab: self.runtime_expandtab.take(),
            language_indent: std::mem::take(&mut self.language_indent),
            autoindent: std::mem::replace(&mut self.autoindent, self.buffer_defaults.autoindent),
            viewport_top: self.viewport.top(),
            last_change: self.last_change.take(),
            undone_change: self.undone_change.take(),
            insert_repeat: self.insert_repeat.take(),
            insert_reopen: std::mem::take(&mut self.insert_reopen),
            insert_record: std::mem::take(&mut self.insert_record),
            insert_moved: std::mem::take(&mut self.insert_moved),
            insert_last_char: self.insert_last_char.take(),
            search_status: self.search_status.take(),
            change_list: std::mem::take(&mut self.change_list),
            change_index: std::mem::take(&mut self.change_index),
        }
    }

    fn restore_state(&mut self, state: ParkedState) {
        self.buffer = state.buffer;
        self.buffers.current_listed = state.listed;
        self.cursor = state.cursor;
        self.file_indent = state.file_indent;
        self.runtime_shiftwidth = state.runtime_shiftwidth;
        self.runtime_expandtab = state.runtime_expandtab;
        self.language_indent = state.language_indent;
        self.autoindent = state.autoindent;
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
        self.change_list = state.change_list;
        self.change_index = state.change_index;
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
        self.change_list.clear();
        self.change_index = 0;
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
        self.change_origin_override = None;
        self.insert_session_origin = None;
        self.insert_run_origin = None;
        self.insert_run_active = false;
        self.command.clear();
    }
}

impl Editor {
    pub fn file_is_open(&self, target: &Path) -> bool {
        self.find_buffer(target).is_some()
    }

    pub fn file_is_current(&self, target: &Path) -> bool {
        self.find_buffer(target) == Some(self.buffers.current())
    }

    pub fn file_snapshot(&self, target: &Path) -> Option<Rope> {
        let number = self.find_buffer(target)?;
        if number == self.buffers.current() {
            return Some(self.buffer.snapshot());
        }
        self.buffers
            .entry(number)
            .map(|entry| entry.state.buffer.snapshot())
    }

    pub fn file_hash(&self, target: &Path) -> Option<u64> {
        let snapshot = self.file_snapshot(target)?;
        Some(content_hash(snapshot.to_string().as_bytes()))
    }

    pub fn file_char_len(&self, target: &Path) -> Option<usize> {
        self.file_snapshot(target)
            .map(|snapshot| snapshot.len_chars())
    }

    pub fn apply_text_edits(&mut self, target: &Path, edits: &[TextEdit]) -> Result<(), EditError> {
        let Some(number) = self.find_buffer(target) else {
            return Err(EditError::InvalidEdit);
        };
        if number == self.buffers.current() {
            self.apply_current_text_edits(edits)
        } else {
            self.apply_parked_text_edits(number, edits)
        }
    }

    fn apply_current_text_edits(&mut self, edits: &[TextEdit]) -> Result<(), EditError> {
        let snapshot = self.buffer.snapshot();
        let updated = apply_text_edits(&snapshot, edits).map_err(|_| EditError::InvalidEdit)?;
        let cursor = super::workspace_edit::remap_cursor(
            self.buffer
                .char_index_of(self.cursor.row(), self.cursor.col()),
            edits,
            &updated,
        );
        self.record_boundary_at(self.cursor.row(), self.cursor.col());
        self.buffer.replace_all(&updated.to_string());
        self.cursor.set_position(cursor.0, cursor.1);
        self.selection = None;
        self.normalize_cursor();
        self.remember_wanted_column();
        self.reveal_cursor();
        self.reveal_cursor_horizontally();
        Ok(())
    }

    fn apply_parked_text_edits(
        &mut self,
        number: usize,
        edits: &[TextEdit],
    ) -> Result<(), EditError> {
        let Some(entry) = self
            .buffers
            .parked
            .iter_mut()
            .find(|entry| entry.number == number)
        else {
            return Err(EditError::InvalidEdit);
        };
        let state = &mut entry.state;
        let snapshot = state.buffer.snapshot();
        let updated = apply_text_edits(&snapshot, edits).map_err(|_| EditError::InvalidEdit)?;
        let cursor = super::workspace_edit::remap_cursor(
            state
                .buffer
                .char_index_of(state.cursor.row(), state.cursor.col()),
            edits,
            &updated,
        );
        state
            .buffer
            .checkpoint(state.cursor.row(), state.cursor.col());
        state.buffer.replace_all(&updated.to_string());
        state.cursor.set_position(cursor.0, cursor.1);
        state.cursor.set_desired(cursor.1);
        Ok(())
    }
}

fn buffer_write_outcome(buffer: &Buffer, created: bool) -> WriteOutcome {
    WriteOutcome {
        lines: buffer.line_count(),
        bytes: buffer.file_bytes(),
        created,
    }
}

fn same_file_path(path: &str, target: &Path) -> bool {
    let absolute = paths::absolute(path, &current_dir());
    if absolute == target {
        return true;
    }
    paths::same_file(&absolute, target)
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
    fn file_is_modified_tracks_current_and_parked_buffers() {
        let dir = temp_dir("file_modified");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        assert!(!e.file_is_modified(Path::new(&file_path(&dir, "a.txt"))));
        type_text(&mut e, "x");
        assert!(e.file_is_modified(Path::new(&file_path(&dir, "b.txt"))));
        open(&mut e, &dir, "a.txt");
        assert!(!e.file_is_modified(Path::new(&file_path(&dir, "a.txt"))));
        type_text(&mut e, "y");
        assert!(e.file_is_modified(Path::new(&file_path(&dir, "a.txt"))));
        assert!(e.file_is_modified(Path::new(&file_path(&dir, "b.txt"))));
    }

    #[test]
    fn reload_files_updates_current_and_parked_buffers() {
        let dir = temp_dir("reload_files");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        write_file(&dir, "a.txt", "changed alpha\n");
        write_file(&dir, "b.txt", "changed bravo\n");
        e.reload_files(&[
            PathBuf::from(file_path(&dir, "a.txt")),
            PathBuf::from(file_path(&dir, "b.txt")),
        ]);
        assert_eq!(lines(&e), vec!["changed bravo"]);
        assert!(!e.is_dirty());
        open(&mut e, &dir, "a.txt");
        assert_eq!(lines(&e), vec!["changed alpha"]);
        assert!(!e.is_dirty());
    }

    #[test]
    fn reload_files_skips_modified_buffers() {
        let dir = temp_dir("reload_dirty");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        type_text(&mut e, "x");
        write_file(&dir, "a.txt", "changed\n");
        e.reload_files(&[PathBuf::from(file_path(&dir, "a.txt"))]);
        assert_eq!(lines(&e), vec!["xalpha"]);
        assert!(e.is_dirty());
    }

    #[cfg(unix)]
    #[test]
    fn file_identity_resolves_symlinks() {
        let dir = temp_dir("symlink");
        write_file(&dir, "a.txt", "alpha\n");
        std::os::unix::fs::symlink(dir.join("a.txt"), dir.join("link.txt")).unwrap();
        let mut e = editor_in(&dir, "a.txt");
        e.edit_file(Some(&file_path(&dir, "link.txt")), false)
            .unwrap();
        assert_eq!(e.buffer_count(), 1);
        type_text(&mut e, "x");
        assert!(e.file_is_modified(Path::new(&file_path(&dir, "link.txt"))));
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
    fn edit_file_places_cursor_on_first_non_blank() {
        let dir = temp_dir("first_non_blank");
        write_file(&dir, "a.txt", "  alpha\nbeta\n");
        write_file(&dir, "b.txt", "gamma\n");
        let mut e = editor_in(&dir, "b.txt");
        open(&mut e, &dir, "a.txt");
        assert_eq!((e.row(), e.col()), (0, 2));
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
    fn open_file_buffer_keeps_an_empty_unnamed_buffer() {
        let dir = temp_dir("open_file_buffer");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = Editor::new(Buffer::from_string(""));
        let path = file_path(&dir, "a.txt");
        e.open_file_buffer(&path).unwrap();
        assert_eq!(e.buffer_number(), 2);
        assert_eq!(e.buffer_count(), 2);
        assert_eq!(e.buffer_path(1), None);
        assert_eq!(e.buffer_path(2), Some(path.as_str()));
        e.open_file_buffer(&path).unwrap();
        assert_eq!(e.buffer_number(), 2);
        assert_eq!(e.buffer_count(), 2);
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
    fn reload_buffer_forces_a_parked_dirty_buffer() {
        let dir = temp_dir("reload_parked_force");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        type_text(&mut e, "x");
        open(&mut e, &dir, "b.txt");
        write_file(&dir, "a.txt", "changed\n");
        e.reload_buffer(1).unwrap();
        assert!(e.buffer_select(1));
        assert_eq!(lines(&e), vec!["changed"]);
        assert!(!e.is_dirty());
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
    fn applies_text_edits_to_the_current_buffer_as_one_change() {
        let dir = temp_dir("workspace_edit_current");
        write_file(&dir, "a.txt", "one two\n");
        let mut e = editor_in(&dir, "a.txt");
        e.apply(Action::JumpTo(0, 5));
        let edits = vec![TextEdit {
            start: 4,
            end: 7,
            text: "three".to_string(),
        }];
        e.apply_text_edits(&PathBuf::from(file_path(&dir, "a.txt")), &edits)
            .unwrap();
        assert_eq!(lines(&e), vec!["one three"]);
        assert_eq!((e.row(), e.col()), (0, 5));
        assert!(e.is_dirty());
        e.apply(Action::Undo);
        assert_eq!(lines(&e), vec!["one two"]);
    }

    #[test]
    fn applies_text_edits_to_parked_buffers() {
        let dir = temp_dir("workspace_edit_parked");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        let edits = vec![TextEdit {
            start: 0,
            end: 5,
            text: "omega".to_string(),
        }];
        e.apply_text_edits(&PathBuf::from(file_path(&dir, "a.txt")), &edits)
            .unwrap();
        assert_eq!(lines(&e), vec!["bravo"]);
        open(&mut e, &dir, "a.txt");
        assert_eq!(lines(&e), vec!["omega"]);
        assert!(e.is_dirty());
    }

    #[test]
    fn reports_open_file_snapshots_and_hashes() {
        let dir = temp_dir("workspace_edit_snapshot");
        write_file(&dir, "a.txt", "alpha\n");
        write_file(&dir, "b.txt", "bravo\n");
        let mut e = editor_in(&dir, "a.txt");
        open(&mut e, &dir, "b.txt");
        let path = PathBuf::from(file_path(&dir, "a.txt"));
        assert!(e.file_is_open(&path));
        assert!(!e.file_is_current(&path));
        assert!(e.file_is_current(&PathBuf::from(file_path(&dir, "b.txt"))));
        assert_eq!(e.file_snapshot(&path).unwrap().to_string(), "alpha\n");
        assert_eq!(e.file_char_len(&path), Some(6));
        assert_eq!(
            e.file_hash(&path),
            Some(kawi_buffer::content_hash(b"alpha\n"))
        );
        assert!(!e.file_is_open(Path::new("/tmp/kawi_missing_file")));
    }

    #[test]
    fn rejects_invalid_text_edits_without_changing_the_buffer() {
        let dir = temp_dir("workspace_edit_invalid");
        write_file(&dir, "a.txt", "alpha\n");
        let mut e = editor_in(&dir, "a.txt");
        let edits = vec![TextEdit {
            start: 5,
            end: 2,
            text: "x".to_string(),
        }];
        assert_eq!(
            e.apply_text_edits(&PathBuf::from(file_path(&dir, "a.txt")), &edits),
            Err(EditError::InvalidEdit)
        );
        assert_eq!(lines(&e), vec!["alpha"]);
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
