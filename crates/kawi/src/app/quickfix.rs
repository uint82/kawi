use kawi_task::{Finding, Severity};
use std::path::PathBuf;

const MAX_QUICKFIX_ENTRIES: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickfixEntry {
    pub path: PathBuf,
    pub line: usize,
    pub column: usize,
    pub severity: Severity,
    pub message: String,
}

impl From<Finding> for QuickfixEntry {
    fn from(finding: Finding) -> Self {
        Self {
            path: finding.path,
            line: finding.line,
            column: finding.column,
            severity: finding.severity,
            message: finding.message,
        }
    }
}

#[derive(Debug, Default)]
pub struct Quickfix {
    entries: Vec<QuickfixEntry>,
    current: Option<usize>,
}

impl Quickfix {
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current = None;
    }

    pub fn push(&mut self, entry: QuickfixEntry) -> bool {
        if self.entries.len() >= MAX_QUICKFIX_ENTRIES {
            return false;
        }
        self.entries.push(entry);
        true
    }

    pub fn entries(&self) -> &[QuickfixEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn advance(&mut self, forward: bool) -> Option<(usize, &QuickfixEntry)> {
        if self.entries.is_empty() {
            return None;
        }
        let index = next_index(self.current, self.entries.len(), forward);
        self.current = Some(index);
        self.entries.get(index).map(|entry| (index, entry))
    }
}

fn next_index(current: Option<usize>, len: usize, forward: bool) -> usize {
    let Some(current) = current else {
        return if forward { 0 } else { len.saturating_sub(1) };
    };
    if forward {
        current.saturating_add(1) % len
    } else {
        current.checked_sub(1).unwrap_or(len.saturating_sub(1))
    }
}

impl super::App {
    pub(super) fn handle_quickfix_command(
        &mut self,
        command: kawi_editor::QuickfixCommand,
    ) -> anyhow::Result<bool> {
        match command {
            kawi_editor::QuickfixCommand::Open => self.open_quickfix_picker(),
            kawi_editor::QuickfixCommand::Next => self.handle_quickfix_navigation(true),
            kawi_editor::QuickfixCommand::Previous => self.handle_quickfix_navigation(false),
            kawi_editor::QuickfixCommand::Close => {
                self.picker = None;
                Ok(false)
            }
        }
    }

    fn open_quickfix_picker(&mut self) -> anyhow::Result<bool> {
        if self.quickfix.is_empty() {
            self.show_message("Quickfix list is empty".to_string());
            return Ok(false);
        }
        let root = self.picker_root();
        let rows = self.quickfix_rows(&root);
        self.pager = None;
        self.picker = Some(super::picker::PickerSession::locations(
            root, "Quickfix", rows,
        ));
        Ok(false)
    }

    fn handle_quickfix_navigation(&mut self, forward: bool) -> anyhow::Result<bool> {
        let Some((index, entry)) = self
            .quickfix
            .advance(forward)
            .map(|(index, entry)| (index, entry.clone()))
        else {
            self.show_message("Quickfix list is empty".to_string());
            return Ok(false);
        };
        let total = self.quickfix.len();
        if let Err(message) =
            self.open_file_location(&entry.path, Some(entry.line - 1), Some(entry.column - 1))
        {
            self.show_message(message);
            return Ok(false);
        }
        self.show_message(format!(
            "Quickfix {}/{}: {}",
            index.saturating_add(1),
            total,
            super::compact_error(&entry.message)
        ));
        Ok(false)
    }

    fn quickfix_rows(&self, root: &std::path::Path) -> Vec<kawi_picker::PickerRow> {
        self.quickfix
            .entries()
            .iter()
            .map(|entry| quickfix_row(root, entry))
            .collect()
    }
}

fn quickfix_row(root: &std::path::Path, entry: &QuickfixEntry) -> kawi_picker::PickerRow {
    let path = entry.path.strip_prefix(root).unwrap_or(&entry.path);
    kawi_picker::PickerRow {
        display: format!(
            "{}:{}:{} {} {}",
            path.display(),
            entry.line,
            entry.column,
            entry.severity.label(),
            entry.message
        ),
        path: entry.path.to_string_lossy().to_string(),
        line: Some(entry.line),
        column: Some(entry.column),
        buffer: None,
    }
}

#[cfg(test)]
mod tests {
    use super::super::App;
    use super::*;
    use crate::keymap::Flavor;
    use kawi_buffer::Buffer;
    use std::fs;

    fn entry(line: usize) -> QuickfixEntry {
        QuickfixEntry {
            path: PathBuf::from("a.rs"),
            line,
            column: 1,
            severity: Severity::Error,
            message: format!("problem {line}"),
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("kawi_quickfix_{}_{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn advances_and_wraps_in_both_directions() {
        let mut quickfix = Quickfix::default();
        for line in 1..=3 {
            quickfix.push(entry(line));
        }
        assert_eq!(quickfix.advance(true).unwrap().0, 0);
        assert_eq!(quickfix.advance(true).unwrap().0, 1);
        assert_eq!(quickfix.advance(true).unwrap().0, 2);
        assert_eq!(quickfix.advance(true).unwrap().0, 0);
        assert_eq!(quickfix.advance(false).unwrap().0, 2);
    }

    #[test]
    fn insertion_stops_at_the_limit() {
        let mut quickfix = Quickfix::default();
        for line in 0..MAX_QUICKFIX_ENTRIES {
            assert!(quickfix.push(entry(line)));
        }
        assert!(!quickfix.push(entry(MAX_QUICKFIX_ENTRIES)));
        assert_eq!(quickfix.len(), MAX_QUICKFIX_ENTRIES);
    }

    #[test]
    fn previous_starts_at_the_end() {
        let mut quickfix = Quickfix::default();
        quickfix.push(entry(1));
        quickfix.push(entry(2));
        assert_eq!(quickfix.advance(false).unwrap().0, 1);
    }

    #[test]
    fn empty_and_cleared_lists_have_no_navigation() {
        let mut quickfix = Quickfix::default();
        assert!(quickfix.advance(true).is_none());
        quickfix.push(entry(1));
        assert!(quickfix.advance(true).is_some());
        quickfix.clear();
        assert!(quickfix.advance(true).is_none());
        assert!(quickfix.entries().is_empty());
    }

    #[test]
    fn navigation_opens_files_and_wraps() {
        let dir = temp_dir("navigation");
        let first = dir.join("a.txt");
        let second = dir.join("b.txt");
        fs::write(&first, "zero\none\n").unwrap();
        fs::write(&second, "only\n").unwrap();
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.quickfix.push(QuickfixEntry {
            path: first.clone(),
            line: 2,
            column: 1,
            severity: Severity::Error,
            message: "first".to_string(),
        });
        app.quickfix.push(QuickfixEntry {
            path: second.clone(),
            line: 1,
            column: 1,
            severity: Severity::Warning,
            message: "second".to_string(),
        });
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Next)
            .unwrap();
        assert_eq!(app.editor.filename(), first.to_string_lossy());
        assert_eq!(app.cursor_position(), (1, 0));
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Next)
            .unwrap();
        assert_eq!(app.editor.filename(), second.to_string_lossy());
        assert!(app.last_message().unwrap().starts_with("Quickfix 2/2"));
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Next)
            .unwrap();
        assert_eq!(app.editor.filename(), first.to_string_lossy());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn open_lists_entries_in_the_picker() {
        let dir = temp_dir("open");
        let file = dir.join("a.txt");
        fs::write(&file, "text\n").unwrap();
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.quickfix.push(QuickfixEntry {
            path: file,
            line: 1,
            column: 1,
            severity: Severity::Error,
            message: "problem".to_string(),
        });
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Open)
            .unwrap();
        let picker = app.picker.as_ref().unwrap();
        assert_eq!(picker.picker().title(), "Quickfix");
        assert_eq!(picker.picker().rows().len(), 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn empty_list_reports_without_opening() {
        let mut app = App::new(Buffer::from_string("text"), Flavor::Vim);
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Next)
            .unwrap();
        assert_eq!(app.last_message(), Some("Quickfix list is empty"));
        app.handle_quickfix_command(kawi_editor::QuickfixCommand::Open)
            .unwrap();
        assert!(app.picker.is_none());
        let _ = fs::remove_dir_all(temp_dir("empty"));
    }
}
