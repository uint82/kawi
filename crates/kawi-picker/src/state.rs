use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::fuzzy::fuzzy_score;
use crate::grep::GrepHit;
use crate::input::TextInput;
use crate::selection::{clamped_selection, selection_at};

const MAX_ROWS: usize = 2_000;
const PAGE_ROWS: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerKind {
    Files,
    Grep,
    Buffers,
    Locations,
    GitBranches,
    GitLog,
    GitCommitFiles,
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
    query: TextInput,
    files: Vec<String>,
    hits: Vec<GrepHit>,
    buffers: Vec<(usize, String)>,
    locations: Vec<PickerRow>,
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

    pub fn locations(title: &'static str, rows: Vec<PickerRow>) -> Self {
        let mut picker = Self::new(PickerKind::Locations, title);
        picker.locations = rows;
        picker.refresh();
        picker
    }

    pub fn git_branches(title: &'static str, rows: Vec<PickerRow>) -> Self {
        let mut picker = Self::new(PickerKind::GitBranches, title);
        picker.locations = rows;
        picker.refresh();
        picker
    }

    pub fn git_log(title: &'static str) -> Self {
        let mut picker = Self::new(PickerKind::GitLog, title);
        picker.loading = true;
        picker
    }

    pub fn git_commit_files(title: &'static str, rows: Vec<PickerRow>) -> Self {
        let mut picker = Self::new(PickerKind::GitCommitFiles, title);
        picker.locations = rows;
        picker.refresh();
        picker
    }

    fn new(kind: PickerKind, title: &'static str) -> Self {
        Self {
            kind,
            title,
            query: TextInput::new(),
            files: Vec::new(),
            hits: Vec::new(),
            buffers: Vec::new(),
            locations: Vec::new(),
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
        self.query.text()
    }

    pub fn cursor(&self) -> usize {
        self.query.cursor()
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

    pub fn needs_more_rows(&self) -> bool {
        self.rows.is_empty()
            || self.selected.unwrap_or_default().saturating_add(PAGE_ROWS) >= self.rows.len()
    }

    pub fn set_locations(&mut self, rows: Vec<PickerRow>) {
        let selected_index = self.selected;
        let selected = self
            .selected
            .and_then(|index| self.rows.get(index))
            .map(location_key);
        self.locations = rows;
        self.rows = self.location_rows();
        self.selected = selected
            .and_then(|selected| {
                self.rows
                    .iter()
                    .position(|row| location_key(row) == selected)
            })
            .or_else(|| selected_index.and_then(|index| selection_at(index, self.rows.len())))
            .or_else(|| (!self.rows.is_empty()).then_some(0));
    }

    pub fn set_status(&mut self, status: Option<String>) {
        self.status = status;
        self.loading = false;
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
        self.query.set_text(query);
        self.after_query_change();
    }

    pub fn insert_text(&mut self, text: &str) {
        self.query.insert_text(text);
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
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => self.navigate(false, 1),
            KeyCode::BackTab => self.navigate(false, 1),
            KeyCode::Tab => self.navigate(true, 1),
            KeyCode::Home => self.set_selected(0),
            KeyCode::End => self.set_selected(self.rows.len().saturating_sub(1)),
            KeyCode::Left => self.move_query(false),
            KeyCode::Right => self.move_query(true),
            KeyCode::Backspace => self.edit_query(TextInput::backspace),
            KeyCode::Delete => self.edit_query(TextInput::delete),
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
                'u' => self.edit_query(TextInput::clear),
                'w' => self.edit_query(TextInput::delete_word),
                _ => PickerOutcome::Consumed,
            };
        }
        if key.modifiers.contains(KeyModifiers::ALT) {
            return PickerOutcome::Consumed;
        }
        self.query.insert_char(character);
        self.after_query_change()
    }

    fn open_selected(&self) -> PickerOutcome {
        match self.selected.and_then(|index| self.rows.get(index)) {
            Some(row) => PickerOutcome::Open(row.clone()),
            None => PickerOutcome::Consumed,
        }
    }

    fn navigate(&mut self, forward: bool, amount: usize) -> PickerOutcome {
        self.selected = clamped_selection(self.selected, self.rows.len(), forward, amount);
        PickerOutcome::Consumed
    }

    fn set_selected(&mut self, index: usize) -> PickerOutcome {
        self.selected = selection_at(index, self.rows.len());
        PickerOutcome::Consumed
    }

    fn move_query(&mut self, forward: bool) -> PickerOutcome {
        if forward {
            self.query.move_right();
        } else {
            self.query.move_left();
        }
        PickerOutcome::Consumed
    }

    fn edit_query(&mut self, edit: impl FnOnce(&mut TextInput)) -> PickerOutcome {
        edit(&mut self.query);
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
            PickerKind::Locations
            | PickerKind::GitBranches
            | PickerKind::GitLog
            | PickerKind::GitCommitFiles => self.refresh_locations(),
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

    fn refresh_locations(&mut self) {
        self.rows = self.location_rows();
    }

    fn location_rows(&self) -> Vec<PickerRow> {
        if self.query().is_empty() {
            return self.locations.iter().take(MAX_ROWS).cloned().collect();
        }
        let candidates = self
            .locations
            .iter()
            .map(|row| (row.display.clone(), row.clone()));
        self.rank_fuzzy(candidates)
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
            .filter_map(|(text, row)| fuzzy_score(self.query(), &text).map(|score| (score, row)))
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

fn location_key(row: &PickerRow) -> (String, Option<usize>, Option<usize>, Option<usize>) {
    (row.path.clone(), row.line, row.column, row.buffer)
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
    fn locations_filter_and_open() {
        let rows = vec![
            PickerRow {
                display: "src/main.rs:1:1".to_string(),
                path: "src/main.rs".to_string(),
                line: Some(1),
                column: Some(1),
                buffer: None,
            },
            PickerRow {
                display: "src/lib.rs:4:2".to_string(),
                path: "src/lib.rs".to_string(),
                line: Some(4),
                column: Some(2),
                buffer: None,
            },
        ];
        let mut picker = Picker::locations("References", rows);
        assert_eq!(picker.kind(), PickerKind::Locations);
        assert_eq!(picker.rows().len(), 2);
        assert_eq!(picker.rows()[0].path, "src/main.rs");
        for character in "lib".chars() {
            picker.handle_key(key(KeyCode::Char(character)));
        }
        assert_eq!(picker.rows().len(), 1);
        let outcome = picker.handle_key(key(KeyCode::Enter));
        assert!(matches!(outcome, PickerOutcome::Open(row) if row.path == "src/lib.rs"));
    }

    #[test]
    fn replacing_locations_preserves_the_selected_row() {
        let rows = vec![
            PickerRow {
                display: "a.rs:1:1 first".to_string(),
                path: "a.rs".to_string(),
                line: Some(1),
                column: Some(1),
                buffer: None,
            },
            PickerRow {
                display: "b.rs:2:1 second".to_string(),
                path: "b.rs".to_string(),
                line: Some(2),
                column: Some(1),
                buffer: None,
            },
        ];
        let mut picker = Picker::locations("Problems", rows.clone());
        picker.handle_key(key(KeyCode::Down));
        let mut updated = vec![PickerRow {
            display: "a.rs:3:1 new".to_string(),
            path: "a.rs".to_string(),
            line: Some(3),
            column: Some(1),
            buffer: None,
        }];
        updated.extend(rows);
        updated[2].display = "b.rs:2:1 updated message".to_string();
        picker.set_locations(updated);
        assert_eq!(picker.selected(), Some(2));
        assert_eq!(picker.rows()[2].display, "b.rs:2:1 updated message");
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

    #[test]
    fn tab_moves_forward_and_backtab_moves_backward() {
        let mut picker = files();
        assert_eq!(picker.selected(), Some(0));
        picker.handle_key(key(KeyCode::Tab));
        assert_eq!(picker.selected(), Some(1));
        picker.handle_key(key(KeyCode::Tab));
        assert_eq!(picker.selected(), Some(2));
        picker.handle_key(key(KeyCode::BackTab));
        assert_eq!(picker.selected(), Some(1));
        picker.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::SHIFT));
        assert_eq!(picker.selected(), Some(0));
    }
}
