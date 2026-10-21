use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::fuzzy::fuzzy_score;
use crate::grep::GrepHit;

const MAX_ROWS: usize = 2_000;
const PAGE_ROWS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Files,
    Grep,
    Buffers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerRow {
    pub display: String,
    pub path: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub buffer: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerOutcome {
    Consumed,
    QueryChanged,
    Open(PickerRow),
    Cancel,
}

pub struct Picker {
    kind: PickerKind,
    title: &'static str,
    query: String,
    cursor: usize,
    files: Vec<String>,
    hits: Vec<GrepHit>,
    buffers: Vec<(usize, String)>,
    rows: Vec<PickerRow>,
    selected: Option<usize>,
    loading: bool,
    status: Option<String>,
}

impl Picker {
    pub fn files() -> Self {
        Self::new(PickerKind::Files, "Files")
    }

    pub fn grep() -> Self {
        Self::new(PickerKind::Grep, "Live Grep")
    }

    pub fn buffers(buffers: Vec<(usize, String)>) -> Self {
        let mut picker = Self::new(PickerKind::Buffers, "Buffers");
        picker.buffers = buffers;
        picker.refresh();
        picker
    }

    fn new(kind: PickerKind, title: &'static str) -> Self {
        Self {
            kind,
            title,
            query: String::new(),
            cursor: 0,
            files: Vec::new(),
            hits: Vec::new(),
            buffers: Vec::new(),
            rows: Vec::new(),
            selected: None,
            loading: false,
            status: None,
        }
    }

    pub fn kind(&self) -> PickerKind {
        self.kind
    }

    pub fn title(&self) -> &'static str {
        self.title
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn rows(&self) -> &[PickerRow] {
        &self.rows
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    pub fn loading(&self) -> bool {
        self.loading
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    pub fn set_files(&mut self, files: Vec<String>) {
        self.files = files;
        self.loading = false;
        self.status = None;
        self.refresh();
    }

    pub fn set_hits(&mut self, hits: Vec<GrepHit>) {
        self.hits = hits;
        self.loading = false;
        self.status = None;
        self.refresh();
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
    }

    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.cursor = self.query.chars().count();
        self.after_query_change();
    }

    pub fn insert_text(&mut self, text: &str) {
        for character in text.chars().filter(|character| !character.is_control()) {
            self.insert(character);
        }
        self.after_query_change();
    }

    pub fn set_error(&mut self, error: String) {
        self.hits.clear();
        self.loading = false;
        self.status = Some(error);
        self.refresh();
    }

    pub fn clear_hits(&mut self) {
        self.hits.clear();
        self.rows.clear();
        self.selected = None;
        self.loading = false;
        self.status = None;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PickerOutcome {
        match key.code {
            KeyCode::Esc => PickerOutcome::Cancel,
            KeyCode::Enter => self.open_selected(),
            KeyCode::Up => self.navigate(false, 1),
            KeyCode::Down => self.navigate(true, 1),
            KeyCode::PageUp => self.navigate(false, PAGE_ROWS),
            KeyCode::PageDown => self.navigate(true, PAGE_ROWS),
            KeyCode::Home => self.set_selected(0),
            KeyCode::End => self.set_selected(self.rows.len().saturating_sub(1)),
            KeyCode::Left => self.move_cursor_left(),
            KeyCode::Right => self.move_cursor_right(),
            KeyCode::Backspace => self.delete_before(),
            KeyCode::Delete => self.delete_at(),
            KeyCode::Char(character) => self.handle_character(key, character),
            _ => PickerOutcome::Consumed,
        }
    }

    fn handle_character(&mut self, key: KeyEvent, character: char) -> PickerOutcome {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match character {
                'c' => PickerOutcome::Cancel,
                'n' | 'j' => self.navigate(true, 1),
                'p' | 'k' => self.navigate(false, 1),
                'u' => self.clear_query(),
                'w' => self.delete_word(),
                _ => PickerOutcome::Consumed,
            };
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            return PickerOutcome::Consumed;
        }
        self.insert(character);
        self.after_query_change()
    }

    fn open_selected(&self) -> PickerOutcome {
        match self.selected.and_then(|index| self.rows.get(index)) {
            Some(row) => PickerOutcome::Open(row.clone()),
            None => PickerOutcome::Consumed,
        }
    }

    fn navigate(&mut self, forward: bool, amount: usize) -> PickerOutcome {
        if self.rows.is_empty() {
            self.selected = None;
            return PickerOutcome::Consumed;
        }
        let length = self.rows.len();
        let current = self.selected.unwrap_or(0);
        let next = if forward {
            current.saturating_add(amount).min(length - 1)
        } else {
            current.saturating_sub(amount)
        };
        self.selected = Some(next);
        PickerOutcome::Consumed
    }

    fn set_selected(&mut self, index: usize) -> PickerOutcome {
        if self.rows.is_empty() {
            self.selected = None;
        } else {
            self.selected = Some(index.min(self.rows.len() - 1));
        }
        PickerOutcome::Consumed
    }

    fn move_cursor_left(&mut self) -> PickerOutcome {
        self.cursor = self.cursor.saturating_sub(1);
        PickerOutcome::Consumed
    }

    fn move_cursor_right(&mut self) -> PickerOutcome {
        self.cursor = self
            .cursor
            .saturating_add(1)
            .min(self.query.chars().count());
        PickerOutcome::Consumed
    }

    fn insert(&mut self, character: char) {
        let byte = byte_index(&self.query, self.cursor);
        self.query.insert(byte, character);
        self.cursor = self.cursor.saturating_add(1);
    }

    fn delete_before(&mut self) -> PickerOutcome {
        if self.cursor == 0 {
            return PickerOutcome::Consumed;
        }
        let start = byte_index(&self.query, self.cursor - 1);
        let end = byte_index(&self.query, self.cursor);
        self.query.replace_range(start..end, "");
        self.cursor = self.cursor.saturating_sub(1);
        self.after_query_change()
    }

    fn delete_at(&mut self) -> PickerOutcome {
        let characters = self.query.chars().count();
        if self.cursor >= characters {
            return PickerOutcome::Consumed;
        }
        let start = byte_index(&self.query, self.cursor);
        let end = byte_index(&self.query, self.cursor + 1);
        self.query.replace_range(start..end, "");
        self.after_query_change()
    }

    fn clear_query(&mut self) -> PickerOutcome {
        self.query.clear();
        self.cursor = 0;
        self.after_query_change()
    }

    fn delete_word(&mut self) -> PickerOutcome {
        if self.cursor == 0 {
            return PickerOutcome::Consumed;
        }
        let characters: Vec<char> = self.query.chars().collect();
        let mut start = self.cursor;
        while start > 0 && characters[start - 1].is_whitespace() {
            start -= 1;
        }
        while start > 0 && !characters[start - 1].is_whitespace() {
            start -= 1;
        }
        let start_byte = byte_index(&self.query, start);
        let end_byte = byte_index(&self.query, self.cursor);
        self.query.replace_range(start_byte..end_byte, "");
        self.cursor = start;
        self.after_query_change()
    }

    fn after_query_change(&mut self) -> PickerOutcome {
        if self.kind == PickerKind::Grep {
            self.hits.clear();
            self.rows.clear();
            self.selected = None;
            self.status = None;
            self.loading = true;
            return PickerOutcome::QueryChanged;
        }
        self.refresh();
        PickerOutcome::Consumed
    }

    fn refresh(&mut self) {
        match self.kind {
            PickerKind::Files => self.refresh_files(),
            PickerKind::Grep => self.refresh_grep(),
            PickerKind::Buffers => self.refresh_buffers(),
        }
        self.selected = if self.rows.is_empty() { None } else { Some(0) };
    }

    fn refresh_files(&mut self) {
        let candidates = self.files.iter().map(|path| {
            (
                path.clone(),
                PickerRow {
                    display: path.clone(),
                    path: path.clone(),
                    line: None,
                    column: None,
                    buffer: None,
                },
            )
        });
        self.rows = self.rank_fuzzy(candidates);
    }

    fn refresh_grep(&mut self) {
        self.rows = self
            .hits
            .iter()
            .take(MAX_ROWS)
            .map(|hit| PickerRow {
                display: format!("{}:{}:{}: {}", hit.path, hit.line, hit.column, hit.text),
                path: hit.path.clone(),
                line: Some(hit.line),
                column: Some(hit.column),
                buffer: None,
            })
            .collect();
    }

    fn refresh_buffers(&mut self) {
        let candidates = self.buffers.iter().map(|(number, name)| {
            (
                name.clone(),
                PickerRow {
                    display: format!("{number}: {name}"),
                    path: name.clone(),
                    line: None,
                    column: None,
                    buffer: Some(*number),
                },
            )
        });
        self.rows = self.rank_fuzzy(candidates);
    }

    fn rank_fuzzy(&self, candidates: impl Iterator<Item = (String, PickerRow)>) -> Vec<PickerRow> {
        let mut ranked: Vec<(i32, PickerRow)> = candidates
            .filter_map(|(text, row)| fuzzy_score(&self.query, &text).map(|score| (score, row)))
            .collect();
        ranked.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then(left.1.display.cmp(&right.1.display))
        });
        ranked
            .into_iter()
            .take(MAX_ROWS)
            .map(|(_, row)| row)
            .collect()
    }
}

fn byte_index(text: &str, character: usize) -> usize {
    text.char_indices()
        .nth(character)
        .map(|(byte, _)| byte)
        .unwrap_or(text.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn files() -> Picker {
        let mut picker = Picker::files();
        picker.set_files(vec![
            "src/main.rs".to_string(),
            "src/lib.rs".to_string(),
            "README.md".to_string(),
        ]);
        picker
    }

    #[test]
    fn files_filter_and_rank_fuzzy_matches() {
        let mut picker = files();
        for character in "main".chars() {
            picker.handle_key(key(KeyCode::Char(character)));
        }
        assert_eq!(picker.rows().len(), 1);
        assert_eq!(picker.rows()[0].path, "src/main.rs");
    }

    #[test]
    fn enter_opens_the_selected_row() {
        let mut picker = files();
        picker.handle_key(key(KeyCode::Down));
        let outcome = picker.handle_key(key(KeyCode::Enter));
        assert!(matches!(outcome, PickerOutcome::Open(row) if row.path == "src/lib.rs"));
    }

    #[test]
    fn query_editing_supports_cursor_and_word_deletion() {
        let mut picker = files();
        for character in "main".chars() {
            picker.handle_key(key(KeyCode::Char(character)));
        }
        picker.handle_key(key(KeyCode::Backspace));
        assert_eq!(picker.query(), "mai");
        picker.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(picker.query(), "");
        for character in "src lib".chars() {
            picker.handle_key(key(KeyCode::Char(character)));
        }
        picker.handle_key(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(picker.query(), "src ");
    }

    #[test]
    fn grep_returns_query_changes_and_builds_rows() {
        let mut picker = Picker::grep();
        let outcome = picker.handle_key(key(KeyCode::Char('n')));
        assert_eq!(outcome, PickerOutcome::QueryChanged);
        assert!(picker.loading());
        picker.set_hits(vec![GrepHit {
            path: "src/main.rs".to_string(),
            line: 3,
            column: 5,
            text: "needle".to_string(),
        }]);
        assert_eq!(picker.rows().len(), 1);
        assert_eq!(picker.rows()[0].display, "src/main.rs:3:5: needle");
    }

    #[test]
    fn buffers_filter_by_name() {
        let mut picker = Picker::buffers(vec![
            (1, "src/main.rs".to_string()),
            (2, "README.md".to_string()),
        ]);
        for character in "read".chars() {
            picker.handle_key(key(KeyCode::Char(character)));
        }
        assert_eq!(picker.rows().len(), 1);
        assert_eq!(picker.rows()[0].buffer, Some(2));
    }

    #[test]
    fn navigation_clamps_at_the_edges() {
        let mut picker = files();
        picker.handle_key(key(KeyCode::Down));
        assert_eq!(picker.selected(), Some(1));
        picker.handle_key(key(KeyCode::Down));
        assert_eq!(picker.selected(), Some(2));
        picker.handle_key(key(KeyCode::Down));
        assert_eq!(picker.selected(), Some(2));
        picker.handle_key(key(KeyCode::Up));
        assert_eq!(picker.selected(), Some(1));
    }

    #[test]
    fn escape_cancels() {
        let mut picker = files();
        assert_eq!(picker.handle_key(key(KeyCode::Esc)), PickerOutcome::Cancel);
    }
}
