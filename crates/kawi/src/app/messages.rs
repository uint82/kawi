use std::collections::VecDeque;

use anyhow::Result;
use kawi_tui::frame::MessageSeverity;

use super::diagnostic::{Diagnostic, DiagnosticCode, DiagnosticSource};
use super::{pager_lines, App};

// 200 messages is enough to reconstruct a session's mistakes without turning :messages into a novel.
const MESSAGE_HISTORY_LIMIT: usize = 200;

#[derive(Default)]
pub(crate) struct MessageHistory {
    entries: VecDeque<Diagnostic>,
}

impl MessageHistory {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, diagnostic: Diagnostic) {
        if self.entries.len() == MESSAGE_HISTORY_LIMIT {
            self.entries.pop_front();
        }
        self.entries.push_back(diagnostic);
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn lines(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for diagnostic in &self.entries {
            lines.extend(pager_lines(&diagnostic.render(), width));
        }
        lines
    }

    #[cfg(test)]
    fn last(&self) -> Option<&Diagnostic> {
        self.entries.back()
    }
}

impl App {
    pub(super) fn show_diagnostic(&mut self, diagnostic: Diagnostic) {
        let severity = diagnostic.severity();
        let text = diagnostic.render();
        self.message_history.push(diagnostic);
        self.present(severity, text);
    }

    pub(super) fn show_message(&mut self, text: String) {
        self.show_diagnostic(Diagnostic::information(text));
    }

    pub(super) fn show_error(&mut self, text: String) {
        self.show_source_failure(DiagnosticSource::Builtin, text);
    }

    pub(super) fn show_error_code(&mut self, code: impl Into<DiagnosticCode>, text: String) {
        self.show_source_error(DiagnosticSource::Builtin, code, text);
    }

    pub(super) fn show_source_error(
        &mut self,
        source: DiagnosticSource,
        code: impl Into<DiagnosticCode>,
        text: String,
    ) {
        self.show_diagnostic(Diagnostic::error(text).with_code(code).with_source(source));
    }

    pub(super) fn show_source_failure(&mut self, source: DiagnosticSource, text: String) {
        self.show_diagnostic(Diagnostic::error(text).with_source(source));
    }

    pub(super) fn show_warning(&mut self, text: String) {
        self.show_diagnostic(Diagnostic::warning(text));
    }

    pub(super) fn show_success(&mut self, text: String) {
        self.show_diagnostic(Diagnostic::success(text));
    }

    pub(super) fn show_report(&mut self, text: String) {
        let diagnostic = Diagnostic::information(text);
        self.present(diagnostic.severity(), diagnostic.render());
    }

    fn present(&mut self, severity: MessageSeverity, text: String) {
        self.hover = None;
        self.message_severity = severity;
        let lines = pager_lines(&text, self.screen_width);
        if lines.len() > 1 {
            self.show_pager(lines);
        } else {
            self.pager = None;
            self.message = Some(text);
        }
    }

    pub(super) fn show_pager(&mut self, lines: Vec<String>) {
        self.hover = None;
        self.plugin_panel.unfocus();
        self.message = None;
        self.pager = Some(lines);
    }

    pub(super) fn clear_message(&mut self) {
        self.message = None;
        self.pager = None;
    }

    pub(super) fn handle_messages_command(&mut self, clear: bool) -> Result<bool> {
        if clear {
            self.message_history.clear();
            self.clear_message();
            return Ok(false);
        }
        if self.message_history.is_empty() {
            self.show_message("No messages".to_string());
            return Ok(false);
        }
        let lines = self.message_history.lines(self.screen_width);
        self.show_pager(lines);
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn information(text: &str) -> Diagnostic {
        Diagnostic::information(text)
    }

    #[test]
    fn history_keeps_order() {
        let mut history = MessageHistory::new();
        history.push(information("first"));
        history.push(information("second"));
        assert_eq!(history.lines(80), ["first", "second"]);
    }

    #[test]
    fn history_stops_at_the_limit() {
        let mut history = MessageHistory::new();
        for index in 0..=MESSAGE_HISTORY_LIMIT {
            history.push(information(&format!("message {index}")));
        }
        let lines = history.lines(80);
        assert_eq!(lines.len(), MESSAGE_HISTORY_LIMIT);
        assert_eq!(lines[0], "message 1");
        assert_eq!(
            lines[MESSAGE_HISTORY_LIMIT - 1],
            format!("message {MESSAGE_HISTORY_LIMIT}")
        );
    }

    #[test]
    fn wraps_long_entries() {
        let mut history = MessageHistory::new();
        history.push(information("abcd"));
        assert_eq!(history.lines(2), ["ab", "cd"]);
    }

    #[test]
    fn clear_empties_history() {
        let mut history = MessageHistory::new();
        history.push(information("first"));
        history.clear();
        assert!(history.is_empty());
    }

    #[test]
    fn history_keeps_the_typed_diagnostic() {
        let mut history = MessageHistory::new();
        history.push(
            Diagnostic::error("nope")
                .with_code("K900")
                .with_source(DiagnosticSource::Git),
        );
        let diagnostic = history.last().expect("diagnostic");
        assert_eq!(diagnostic.severity(), MessageSeverity::Error);
        assert_eq!(diagnostic.code().map(DiagnosticCode::as_str), Some("K900"));
        assert_eq!(diagnostic.source(), DiagnosticSource::Git);
        assert_eq!(diagnostic.render(), "K900: nope");
    }
}
