use super::diagnostic_ui::SourceDisplay;
use super::problems::Problem;
use super::App;
use anyhow::Result;
use crossterm::event::KeyEvent;

pub(super) struct DiagnosticPopup {
    entries: Vec<Problem>,
    show_source: bool,
    scroll: usize,
}

impl DiagnosticPopup {
    pub(super) fn new(entries: Vec<Problem>, show_source: bool) -> Self {
        Self {
            entries,
            show_source,
            scroll: 0,
        }
    }

    pub(super) fn entries(&self) -> &[Problem] {
        &self.entries
    }

    pub(super) fn show_source(&self) -> bool {
        self.show_source
    }

    pub(super) fn scroll(&self) -> usize {
        self.scroll
    }

    pub(super) fn scroll_by(&mut self, delta: isize, limit: usize) {
        self.scroll = self.scroll.saturating_add_signed(delta).min(limit);
    }
}

impl App {
    pub(super) fn show_diagnostic_popup(&mut self) -> Result<bool> {
        self.diagnostic_popup = None;
        let Some(path) = self.problem_path() else {
            self.show_message("No diagnostics".to_string());
            return Ok(false);
        };
        let row = self.editor.row();
        let column = self.editor.col();
        let mut entries = self.problems.diagnostics_at(&path, row);
        if entries.is_empty() {
            self.show_message("No diagnostics".to_string());
            return Ok(false);
        }
        let primary = self.problems.primary_diagnostic(&path, row, Some(column));
        move_primary_first(&mut entries, primary.as_ref());
        let show_source =
            self.diagnostic_options.source == SourceDisplay::Always || source_count(&entries) > 1;
        self.pager = None;
        self.hover = None;
        self.diagnostic_popup = Some(DiagnosticPopup::new(entries, show_source));
        Ok(false)
    }

    pub(super) fn handle_diagnostic_popup_key(&mut self, key: KeyEvent) -> Result<Option<bool>> {
        let up = self.completion_options.keys.up.event();
        let down = self.completion_options.keys.down.event();
        let geometry = (key == up || key == down)
            .then(|| self.diagnostic_scroll())
            .flatten();
        Ok(super::overlay_scroll::handle_scroll_key(
            super::overlay_scroll::ScrollKey {
                session: &mut self.diagnostic_popup,
                geometry,
                key,
                up,
                down,
            },
        ))
    }

    pub(super) fn close_diagnostic_popup(&mut self) -> bool {
        self.diagnostic_popup.take().is_some()
    }
}

fn move_primary_first(entries: &mut Vec<Problem>, primary: Option<&Problem>) {
    let Some(primary) = primary else {
        return;
    };
    let Some(index) = entries.iter().position(|entry| entry == primary) else {
        return;
    };
    let primary = entries.remove(index);
    entries.insert(0, primary);
}

fn source_count(entries: &[Problem]) -> usize {
    entries
        .iter()
        .map(|entry| entry.source.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}

impl super::overlay_scroll::Scrollable for DiagnosticPopup {
    fn scroll_by(&mut self, delta: isize, limit: usize) {
        DiagnosticPopup::scroll_by(self, delta, limit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::problems::ProblemSeverity;
    use crate::keymap::Flavor;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use kawi_buffer::Buffer;
    use kawi_lsp::{Diagnostic, DiagnosticPosition, DiagnosticRange, DiagnosticSeverity};
    use std::path::PathBuf;

    fn diagnostic(
        line: u32,
        start: u32,
        end: u32,
        severity: DiagnosticSeverity,
        message: &str,
    ) -> Diagnostic {
        Diagnostic {
            range: DiagnosticRange {
                start: DiagnosticPosition {
                    line,
                    character: start,
                },
                end: DiagnosticPosition {
                    line,
                    character: end,
                },
            },
            severity,
            message: message.to_string(),
            source: Some("fake".to_string()),
            code: None,
            code_description: None,
            tags: Vec::new(),
            related: Vec::new(),
        }
    }

    fn app_with_problem() -> App {
        let path = std::env::current_dir().expect("cwd").join("popup.rs");
        let mut buffer = Buffer::from_string("value\n");
        buffer.path = Some("popup.rs".to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.problems.replace_lsp_test(
            "fake",
            &path,
            vec![diagnostic(0, 0, 5, DiagnosticSeverity::Error, "broken")],
        );
        app
    }

    fn problem(source: &str, column: usize, severity: ProblemSeverity, message: &str) -> Problem {
        Problem {
            source: source.to_string(),
            path: PathBuf::from("/tmp/popup.rs"),
            line: 1,
            column,
            end_line: 1,
            end_column: column.saturating_add(1),
            severity,
            message: message.to_string(),
            code: None,
            code_description: None,
            provider: None,
            tags: Vec::new(),
            related: Vec::new(),
        }
    }

    #[test]
    fn primary_moves_to_the_front_without_reordering_the_rest() {
        let mut entries = vec![
            problem("lsp:a", 1, ProblemSeverity::Error, "first"),
            problem("lsp:a", 6, ProblemSeverity::Warning, "cursor"),
            problem("lsp:a", 9, ProblemSeverity::Error, "later"),
        ];
        let primary = entries[1].clone();
        move_primary_first(&mut entries, Some(&primary));
        let messages: Vec<&str> = entries.iter().map(|entry| entry.message.as_str()).collect();
        assert_eq!(messages, ["cursor", "first", "later"]);
    }

    #[test]
    fn popup_uses_source_count_for_if_many() {
        let entries = [
            problem("lsp:a", 1, ProblemSeverity::Error, "one"),
            problem("lsp:b", 2, ProblemSeverity::Error, "two"),
        ];
        assert_eq!(source_count(&entries), 2);
        let entries = [problem("lsp:a", 1, ProblemSeverity::Error, "one")];
        assert_eq!(source_count(&entries), 1);
    }

    #[test]
    fn leader_opens_the_diagnostic_popup() {
        let mut app = app_with_problem();
        app.handle_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE))
            .expect("leader");
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE))
            .expect("group");
        app.handle_key(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE))
            .expect("popup");
        let popup = app.diagnostic_popup.as_ref().expect("popup");
        assert_eq!(popup.entries().len(), 1);
        assert_eq!(popup.entries()[0].message, "broken");
        app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE))
            .expect("close");
        assert!(app.diagnostic_popup.is_none());
    }

    #[test]
    fn popup_places_the_cursor_diagnostic_first_then_sorts_ties() {
        let path = std::env::current_dir().expect("cwd").join("order.rs");
        let mut buffer = Buffer::from_string("value\n");
        buffer.path = Some("order.rs".to_string());
        let mut app = App::new(buffer, Flavor::Vim);
        app.problems.replace_lsp_test(
            "a",
            &path,
            vec![
                diagnostic(0, 0, 1, DiagnosticSeverity::Error, "first"),
                diagnostic(0, 4, 5, DiagnosticSeverity::Warning, "cursor"),
            ],
        );
        app.problems.replace_lsp_test(
            "b",
            &path,
            vec![diagnostic(0, 0, 1, DiagnosticSeverity::Error, "tie")],
        );
        app.editor.apply(kawi_editor::Action::JumpTo(0, 4));
        app.show_diagnostic_popup().expect("popup");
        let popup = app.diagnostic_popup.as_ref().expect("popup");
        let messages: Vec<&str> = popup
            .entries()
            .iter()
            .map(|entry| entry.message.as_str())
            .collect();
        assert_eq!(messages, ["cursor", "first", "tie"]);
        assert!(popup.show_source());
    }

    #[test]
    fn popup_dismisses_and_scrolls() {
        let mut app = App::new(Buffer::from_string("value\n"), Flavor::Vim);
        app.diagnostic_popup = Some(DiagnosticPopup::new(
            vec![problem("lsp:a", 1, ProblemSeverity::Error, "one")],
            false,
        ));
        assert!(app.close_diagnostic_popup());
        assert!(app.diagnostic_popup.is_none());
    }
}
