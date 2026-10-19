use std::collections::VecDeque;

use super::Editor;
use crate::mode::Mode;

pub const DEFAULT_HISTORY_LIMIT: usize = 10_000;
pub const MAX_HISTORY_LIMIT: usize = 10_000;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct HistoryEntries {
    pub command: Vec<String>,
    pub search: Vec<String>,
}

#[derive(Debug)]
pub(super) struct PromptHistory {
    entries: VecDeque<String>,
    limit: usize,
    pending: Option<String>,
    index: Option<usize>,
}

impl PromptHistory {
    pub(super) fn new(limit: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            limit,
            pending: None,
            index: None,
        }
    }

    pub(super) fn set_limit(&mut self, limit: usize) {
        self.limit = limit;
        self.trim();
    }

    pub(super) fn record(&mut self, text: &str) {
        self.reset_browse();
        if text.is_empty() || self.limit == 0 {
            return;
        }
        if let Some(position) = self.entries.iter().position(|entry| entry == text) {
            self.entries.remove(position);
        }
        self.entries.push_back(text.to_string());
        self.trim();
    }

    pub(super) fn load(&mut self, entries: Vec<String>) {
        self.reset_browse();
        self.entries = entries.into();
        self.trim();
    }

    pub(super) fn snapshot(&self) -> Vec<String> {
        self.entries.iter().cloned().collect()
    }

    pub(super) fn reset_browse(&mut self) {
        self.pending = None;
        self.index = None;
    }

    pub(super) fn mark_edited(&mut self) {
        self.pending = None;
    }

    pub(super) fn older(&mut self, current: &str) -> Option<String> {
        if self.entries.is_empty() {
            return None;
        }
        if self.pending.is_none() {
            self.pending = Some(current.to_string());
        }
        let prefix = self.pending.clone().unwrap_or_default();
        let start = self.index.unwrap_or(self.entries.len());
        for candidate in (0..start).rev() {
            if self.entries[candidate].starts_with(&prefix) {
                self.index = Some(candidate);
                return Some(self.entries[candidate].clone());
            }
        }
        None
    }

    pub(super) fn newer(&mut self, current: &str) -> Option<String> {
        if self.pending.is_none() {
            self.pending = Some(current.to_string());
        }
        let index = self.index?;
        if index + 1 == self.entries.len() {
            self.index = None;
            return self.pending.clone();
        }
        let prefix = self.pending.clone().unwrap_or_default();
        for candidate in index + 1..self.entries.len() {
            if self.entries[candidate].starts_with(&prefix) {
                self.index = Some(candidate);
                return Some(self.entries[candidate].clone());
            }
        }
        None
    }

    fn trim(&mut self) {
        while self.entries.len() > self.limit {
            self.entries.pop_front();
        }
    }
}

impl Editor {
    pub(super) fn history_older(&mut self) {
        self.browse_history(true);
    }

    pub(super) fn history_newer(&mut self) {
        self.browse_history(false);
    }

    fn browse_history(&mut self, older: bool) {
        match self.mode {
            Mode::Command => self.browse_command_history(older),
            Mode::Search => self.browse_search_history(older),
            _ => {}
        }
    }

    fn browse_command_history(&mut self, older: bool) {
        if self.completion.is_some() {
            self.complete_command(older);
            return;
        }
        let current = self.command.text().to_string();
        if let Some(entry) = recall(&mut self.command_history, &current, older) {
            self.command.replace(&entry);
        }
    }

    fn browse_search_history(&mut self, older: bool) {
        let current = self.search.text().to_string();
        if let Some(entry) = recall(&mut self.search_history, &current, older) {
            self.search.replace(&entry);
            self.search_whole_word = false;
            self.jump_to_first_result();
        }
    }

    pub(super) fn record_command_history(&mut self) {
        let text = self.command.text().to_string();
        self.command_history.record(&text);
    }

    pub(super) fn record_search_history(&mut self) {
        let text = self.search.text().to_string();
        self.search_history.record(&text);
    }

    pub fn set_history_limit(&mut self, limit: usize) {
        self.command_history.set_limit(limit);
        self.search_history.set_limit(limit);
    }

    pub fn history_entries(&self) -> HistoryEntries {
        HistoryEntries {
            command: self.command_history.snapshot(),
            search: self.search_history.snapshot(),
        }
    }

    pub fn load_history(&mut self, entries: HistoryEntries) {
        self.command_history.load(entries.command);
        self.search_history.load(entries.search);
    }
}

fn recall(history: &mut PromptHistory, current: &str, older: bool) -> Option<String> {
    if older {
        history.older(current)
    } else {
        history.newer(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;

    fn history(entries: &[&str]) -> PromptHistory {
        let mut history = PromptHistory::new(DEFAULT_HISTORY_LIMIT);
        for entry in entries {
            history.record(entry);
        }
        history.reset_browse();
        history
    }

    #[test]
    fn record_moves_duplicates_to_front() {
        let history = history(&["a", "b", "a"]);
        assert_eq!(history.snapshot(), vec!["b", "a"]);
    }

    #[test]
    fn record_ignores_empty_text() {
        let history = history(&["a", ""]);
        assert_eq!(history.snapshot(), vec!["a"]);
    }

    #[test]
    fn record_respects_limit() {
        let mut history = PromptHistory::new(2);
        for entry in ["a", "b", "c"] {
            history.record(entry);
        }
        assert_eq!(history.snapshot(), vec!["b", "c"]);
    }

    #[test]
    fn zero_limit_disables_history() {
        let mut history = PromptHistory::new(0);
        history.record("a");
        assert_eq!(history.snapshot(), Vec::<String>::new());
        assert_eq!(history.older(""), None);
    }

    #[test]
    fn older_walks_from_newest_and_stops_at_oldest() {
        let mut history = history(&["a", "b", "c"]);
        assert_eq!(history.older(""), Some("c".to_string()));
        assert_eq!(history.older("c"), Some("b".to_string()));
        assert_eq!(history.older("b"), Some("a".to_string()));
        assert_eq!(history.older("a"), None);
    }

    #[test]
    fn newer_restores_pending_text() {
        let mut history = history(&["echo 1", "echo 2"]);
        assert_eq!(history.older("echo"), Some("echo 2".to_string()));
        assert_eq!(history.older("echo 2"), Some("echo 1".to_string()));
        assert_eq!(history.newer("echo 1"), Some("echo 2".to_string()));
        assert_eq!(history.newer("echo 2"), Some("echo".to_string()));
        assert_eq!(history.newer("echo"), None);
    }

    #[test]
    fn browsing_skips_entries_without_the_prefix() {
        let mut history = history(&["set nu", "echo 1", "echo 2"]);
        assert_eq!(history.older("e"), Some("echo 2".to_string()));
        assert_eq!(history.older("echo 2"), Some("echo 1".to_string()));
        assert_eq!(history.older("echo 1"), None);
        assert_eq!(history.newer("echo 1"), Some("echo 2".to_string()));
    }

    #[test]
    fn browsing_without_match_keeps_the_line() {
        let mut history = history(&["echo 1"]);
        assert_eq!(history.older("xyz"), None);
        assert_eq!(history.newer("xyz"), None);
    }

    #[test]
    fn editing_clears_the_pending_prefix() {
        let mut history = history(&["echo 1", "echo 2"]);
        assert_eq!(history.older(""), Some("echo 2".to_string()));
        history.mark_edited();
        assert_eq!(history.older("echo 2x"), None);
    }

    #[test]
    fn editing_at_newest_restores_the_edited_line() {
        let mut history = history(&["echo 1"]);
        assert_eq!(history.older(""), Some("echo 1".to_string()));
        history.mark_edited();
        assert_eq!(history.newer("echo 1x"), Some("echo 1x".to_string()));
    }

    #[test]
    fn recording_resets_browsing() {
        let mut history = history(&["a", "b"]);
        assert_eq!(history.older(""), Some("b".to_string()));
        history.record("c");
        assert_eq!(history.older(""), Some("c".to_string()));
    }

    #[test]
    fn older_on_empty_history_does_nothing() {
        let mut history = PromptHistory::new(DEFAULT_HISTORY_LIMIT);
        assert_eq!(history.older("x"), None);
        assert_eq!(history.newer("x"), None);
    }

    #[test]
    fn load_keeps_the_newest_entries_within_limit() {
        let mut history = PromptHistory::new(2);
        history.load(vec!["a".to_string(), "b".to_string(), "c".to_string()]);
        assert_eq!(history.snapshot(), vec!["b", "c"]);
    }

    fn editor_with(text: &str) -> Editor {
        Editor::new(kawi_buffer::Buffer::from_string(text))
    }

    fn record_command(editor: &mut Editor, text: &str) {
        editor.enter_command();
        for character in text.chars() {
            editor.push_command_char(character);
        }
        editor.cancel_command();
    }

    #[test]
    fn leaving_command_prompt_records_the_line() {
        let mut editor = editor_with("");
        record_command(&mut editor, "write");
        assert_eq!(editor.history_entries().command, vec!["write"]);
    }

    #[test]
    fn empty_command_prompt_is_not_recorded() {
        let mut editor = editor_with("");
        record_command(&mut editor, "");
        assert_eq!(editor.history_entries().command, Vec::<String>::new());
    }

    #[test]
    fn cancelled_command_is_recorded() {
        let mut editor = editor_with("");
        editor.enter_command();
        for character in "aborted".chars() {
            editor.push_command_char(character);
        }
        editor.cancel_command();
        assert_eq!(editor.history_entries().command, vec!["aborted"]);
    }

    #[test]
    fn older_recalls_into_the_command_prompt() {
        let mut editor = editor_with("");
        record_command(&mut editor, "echo 1");
        record_command(&mut editor, "echo 2");
        editor.enter_command();
        editor.apply(Action::HistoryOlder);
        assert_eq!(editor.command_text(), "echo 2");
        editor.apply(Action::HistoryOlder);
        assert_eq!(editor.command_text(), "echo 1");
        editor.apply(Action::HistoryNewer);
        assert_eq!(editor.command_text(), "echo 2");
        editor.apply(Action::HistoryNewer);
        assert_eq!(editor.command_text(), "");
    }

    #[test]
    fn history_keys_feed_the_open_completion() {
        let mut editor = editor_with("");
        record_command(&mut editor, "write");
        editor.enter_command();
        editor.push_command_char('w');
        editor.apply(Action::CompleteCommand);
        assert_eq!(editor.command_text(), "w");
        editor.apply(Action::CompleteCommand);
        assert_eq!(editor.command_text(), "w!");
        editor.apply(Action::HistoryOlder);
        assert_eq!(editor.command_text(), "w");
    }

    #[test]
    fn confirming_and_cancelling_record_searches() {
        let mut editor = editor_with("alpha beta");
        editor.enter_search();
        for character in "alpha".chars() {
            editor.push_search_char(character);
        }
        editor.confirm_search();
        editor.enter_search();
        for character in "cancelled".chars() {
            editor.push_search_char(character);
        }
        editor.cancel_search();
        assert_eq!(editor.history_entries().search, vec!["alpha", "cancelled"]);
    }

    #[test]
    fn empty_search_prompt_is_not_recorded() {
        let mut editor = editor_with("alpha");
        editor.enter_search();
        editor.confirm_search();
        assert_eq!(editor.history_entries().search, Vec::<String>::new());
    }

    #[test]
    fn older_recalls_a_search_and_previews_it() {
        let mut editor = editor_with("alpha beta");
        editor.enter_search();
        for character in "beta".chars() {
            editor.push_search_char(character);
        }
        editor.confirm_search();
        editor.apply(Action::MoveToLineStart);
        editor.enter_search();
        editor.apply(Action::HistoryOlder);
        assert_eq!(editor.search_prompt(), Some("beta"));
        assert_eq!((editor.row(), editor.col()), (0, 6));
    }

    #[test]
    fn search_browsing_without_prefix_match_keeps_the_line() {
        let mut editor = editor_with("alpha beta");
        editor.enter_search();
        for character in "beta".chars() {
            editor.push_search_char(character);
        }
        editor.confirm_search();
        editor.enter_search();
        for character in "zz".chars() {
            editor.push_search_char(character);
        }
        editor.apply(Action::HistoryOlder);
        assert_eq!(editor.search_prompt(), Some("zz"));
    }

    #[test]
    fn searching_a_word_records_it() {
        let mut editor = editor_with("alpha beta");
        editor.search_word(true).unwrap();
        assert_eq!(editor.history_entries().search, vec!["\\<alpha\\>"]);
    }

    #[test]
    fn history_keys_do_nothing_outside_prompts() {
        let mut editor = editor_with("alpha");
        editor.apply(Action::HistoryOlder);
        editor.apply(Action::HistoryNewer);
        assert_eq!((editor.row(), editor.col()), (0, 0));
        assert_eq!(editor.command_prompt(), None);
        assert_eq!(editor.search_prompt(), None);
    }
}
