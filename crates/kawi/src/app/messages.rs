use std::collections::VecDeque;

use anyhow::Result;
use kawi_tui::frame::MessageSeverity;

use super::{pager_lines, App};

// 200 messages is enough to reconstruct a session's mistakes without turning :messages into a novel.
const MESSAGE_HISTORY_LIMIT: usize = 200;

#[derive(Default)]
pub(crate) struct MessageHistory {
    entries: VecDeque<String>,
}

impl MessageHistory {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, text: String) {
        if self.entries.len() == MESSAGE_HISTORY_LIMIT {
            self.entries.pop_front();
        }
        self.entries.push_back(text);
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn lines(&self, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        for text in &self.entries {
            lines.extend(pager_lines(text, width));
        }
        lines
    }
}

impl App {
    pub(super) fn show_message(&mut self, text: String) {
        let severity = infer_severity(&text);
        self.push_message(severity, text);
    }

    pub(super) fn show_error(&mut self, text: String) {
        self.push_message(MessageSeverity::Error, text);
    }

    pub(super) fn show_warning(&mut self, text: String) {
        self.push_message(MessageSeverity::Warning, text);
    }

    pub(super) fn show_success(&mut self, text: String) {
        self.push_message(MessageSeverity::Success, text);
    }

    fn push_message(&mut self, severity: MessageSeverity, text: String) {
        self.message_history.push(text.clone());
        self.present_message(severity, text);
    }

    pub(super) fn show_report(&mut self, text: String) {
        self.present_message(MessageSeverity::Information, text);
    }

    fn present_message(&mut self, severity: MessageSeverity, text: String) {
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

fn infer_severity(text: &str) -> MessageSeverity {
    if is_error(text) {
        MessageSeverity::Error
    } else if is_warning(text) {
        MessageSeverity::Warning
    } else {
        MessageSeverity::Information
    }
}

fn is_error(text: &str) -> bool {
    has_code(text, 'E') || has_code(text, 'K') || starts_with_label(text, "error:")
}

fn is_warning(text: &str) -> bool {
    has_code(text, 'W') || starts_with_label(text, "warning:")
}

fn has_code(text: &str, prefix: char) -> bool {
    text.strip_prefix(prefix).is_some_and(|rest| {
        rest.starts_with(':')
            || rest
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_digit())
    })
}

fn starts_with_label(text: &str, label: &str) -> bool {
    text.get(..label.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(label))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_error_codes() {
        assert_eq!(infer_severity("E492: nope"), MessageSeverity::Error);
        assert_eq!(infer_severity("E: nope"), MessageSeverity::Error);
        assert_eq!(infer_severity("K001: nope"), MessageSeverity::Error);
        assert_eq!(infer_severity("Error: nope"), MessageSeverity::Error);
    }

    #[test]
    fn infers_warning_codes() {
        assert_eq!(infer_severity("W10: careful"), MessageSeverity::Warning);
        assert_eq!(infer_severity("warning: careful"), MessageSeverity::Warning);
    }

    #[test]
    fn leaves_other_text_information() {
        assert_eq!(infer_severity("saved"), MessageSeverity::Information);
        assert_eq!(
            infer_severity("Edit complete"),
            MessageSeverity::Information
        );
    }

    #[test]
    fn history_keeps_order() {
        let mut history = MessageHistory::new();
        history.push("first".to_string());
        history.push("second".to_string());
        assert_eq!(history.lines(80), ["first", "second"]);
    }

    #[test]
    fn history_stops_at_the_limit() {
        let mut history = MessageHistory::new();
        for index in 0..=MESSAGE_HISTORY_LIMIT {
            history.push(format!("message {index}"));
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
        history.push("abcd".to_string());
        assert_eq!(history.lines(2), ["ab", "cd"]);
    }

    #[test]
    fn clear_empties_history() {
        let mut history = MessageHistory::new();
        history.push("first".to_string());
        history.clear();
        assert!(history.is_empty());
    }
}
