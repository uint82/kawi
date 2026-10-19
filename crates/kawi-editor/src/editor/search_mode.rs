use super::{Editor, SearchStatus};
use crate::matcher::Query;
use crate::mode::Mode;
use crate::search::{self, SearchOutcome};

impl Editor {
    pub(super) fn cancel_search(&mut self) {
        self.record_search_history();
        if !self.search.text().is_empty() {
            self.search_last = self.search.text().to_string();
        }
        if let Some((row, col)) = self.search_origin {
            self.cursor.set_position(row, col);
            self.viewport
                .place_top(self.search_view_top, self.buffer.line_count());
        }
        self.search_origin = None;
        self.search_status = None;
        self.mode = Mode::Normal;
    }

    pub fn confirm_search(&mut self) -> Option<SearchOutcome> {
        if self.mode == Mode::Search {
            self.mode = Mode::Normal;
        }
        if self.search.text().is_empty() {
            if self.search_last.is_empty() {
                self.search_status = None;
                return self.missing_search_outcome();
            }
            let last = self.search_last.clone();
            self.search.replace(&last);
        } else {
            self.record_search_history();
            self.search_last = self.search.text().to_string();
        }
        self.search_forward = self.search_prompt_forward;
        match self.execute_search() {
            SearchOutcome::Found { wrapped } => {
                if let Some(origin) = self.search_origin {
                    self.set_pcmark_at(origin);
                }
                Some(SearchOutcome::Found { wrapped })
            }
            SearchOutcome::Missing => self.missing_search_outcome(),
        }
    }

    fn execute_search(&mut self) -> SearchOutcome {
        if let Some((row, col)) = self.search_origin {
            self.cursor.set_position(row, col);
        }
        self.viewport
            .place_top(self.search_view_top, self.buffer.line_count());
        if self.search_forward {
            self.search_next()
        } else {
            self.search_prev()
        }
    }

    fn missing_search_outcome(&mut self) -> Option<SearchOutcome> {
        if let Some((row, col)) = self.search_origin {
            self.cursor.set_position(row, col);
            self.viewport
                .place_top(self.search_view_top, self.buffer.line_count());
            self.remember_wanted_column();
        }
        Some(SearchOutcome::Missing)
    }

    pub(super) fn delete_search_char(&mut self) {
        if self.mode != Mode::Search {
            return;
        }
        self.search_history.mark_edited();
        self.search.pop();
        if self.search.text().is_empty() {
            self.cancel_search();
            return;
        }
        self.jump_to_first_result();
    }

    pub(super) fn enter_search(&mut self, forward: bool) {
        self.search_prompt_forward = forward;
        self.search_view_top = self.viewport.top();
        self.search_origin = Some((self.cursor.row(), self.cursor.col()));
        self.search.clear();
        self.search_history.reset_browse();
        self.search_no_smartcase = false;
        self.search_status = None;
        self.mode = Mode::Search;
    }

    fn highlight_target(&self) -> Option<(&str, (usize, usize))> {
        if self.mode == Mode::Search && !self.search.text().is_empty() {
            return Some((self.search.text(), (self.cursor.row(), self.cursor.col())));
        }
        match &self.search_status {
            Some(status) if status.revision == self.buffer.revision() => {
                Some((self.search.text(), status.current))
            }
            _ => None,
        }
    }

    pub(super) fn jump_to_first_result(&mut self) {
        let origin = self
            .search_origin
            .unwrap_or((self.cursor.row(), self.cursor.col()));
        let query = self.search_query();
        let found = if self.search_prompt_forward {
            let from = (origin.0, origin.1.saturating_add(1));
            search::find_first_from(&self.buffer, &query, from.0, from.1)
                .or_else(|| search::find_first_from(&self.buffer, &query, 0, 0))
        } else {
            search::find_prev(&self.buffer, &query, origin.0, origin.1).map(|(pos, _)| pos)
        };
        self.viewport
            .place_top(self.search_view_top, self.buffer.line_count());
        match found {
            Some((row, col)) => self.cursor.set_position(row, col),
            None => self.cursor.set_position(origin.0, origin.1),
        }
        self.remember_wanted_column();
    }

    fn mark_current(
        &self,
        mut span: search::MatchSpan,
        anchor: (usize, usize),
    ) -> search::MatchSpan {
        span.current = (span.row, span.start) == anchor;
        span
    }

    pub fn next_match(&mut self) -> SearchOutcome {
        self.match_step(true)
    }

    fn place_search_result(&mut self, row: usize, col: usize, wrapped: bool) -> SearchOutcome {
        self.cursor.set_position(row, col);
        self.remember_wanted_column();
        self.refresh_search_status();
        SearchOutcome::Found { wrapped }
    }

    pub fn prev_match(&mut self) -> SearchOutcome {
        self.match_step(false)
    }

    fn match_step(&mut self, same_direction: bool) -> SearchOutcome {
        let before = (self.cursor.row(), self.cursor.col());
        let forward = self.search_forward == same_direction;
        let outcome = if forward {
            self.search_next()
        } else {
            self.search_prev()
        };
        if matches!(outcome, SearchOutcome::Found { .. }) {
            self.set_pcmark_at(before);
        }
        outcome
    }

    pub(super) fn push_search_char(&mut self, c: char) {
        if self.mode != Mode::Search {
            return;
        }
        self.search_history.mark_edited();
        self.search.push(c);
        self.jump_to_first_result();
    }

    fn refresh_search_status(&mut self) {
        let pattern = self.search.text().to_string();
        if pattern.is_empty() {
            self.search_status = None;
            return;
        }
        let revision = self.buffer.revision();
        let query = self.search_query();
        let total = match &self.search_status {
            Some(status)
                if status.pattern == pattern
                    && status.revision == revision
                    && status.no_smartcase == self.search_no_smartcase =>
            {
                status.total
            }
            _ => search::count_matches(&self.buffer, &query),
        };
        if total == 0 {
            self.search_status = None;
            return;
        }
        let index = search::match_rank(
            &self.buffer,
            &query,
            self.cursor.row(),
            self.cursor.col(),
            total,
        );
        self.search_status = Some(SearchStatus {
            pattern,
            no_smartcase: self.search_no_smartcase,
            index,
            total,
            revision,
            current: (self.cursor.row(), self.cursor.col()),
        });
    }

    pub fn search_next(&mut self) -> SearchOutcome {
        self.step_search(true)
    }

    pub fn search_prev(&mut self) -> SearchOutcome {
        self.step_search(false)
    }

    pub fn set_ignorecase(&mut self, ignorecase: bool) {
        self.ignorecase = ignorecase;
        self.search_status = None;
    }

    pub fn set_smartcase(&mut self, smartcase: bool) {
        self.smartcase = smartcase;
        self.search_status = None;
    }

    fn search_sensitive(&self, pattern: &str) -> bool {
        if !self.ignorecase {
            return true;
        }
        let smartcase = self.smartcase && !self.search_no_smartcase;
        if smartcase && pattern.chars().any(char::is_uppercase) {
            return true;
        }
        false
    }

    fn search_query(&self) -> Query {
        self.query_for(self.search.text())
    }

    fn query_for(&self, pattern: &str) -> Query {
        let (literal, whole_word) = search::strip_word_atoms(pattern);
        Query::new(&literal, self.search_sensitive(pattern), whole_word)
    }

    pub fn search_spans(&self, top: usize, height: usize) -> Vec<search::MatchSpan> {
        let (pattern, anchor) = match self.highlight_target() {
            Some(target) => target,
            None => return Vec::new(),
        };
        let query = self.query_for(pattern);
        search::collect_matches(&self.buffer, &query, top, top.saturating_add(height))
            .into_iter()
            .map(|span| self.mark_current(span, anchor))
            .collect()
    }

    fn step_search(&mut self, forward: bool) -> SearchOutcome {
        if self.search.text().is_empty() {
            return SearchOutcome::Missing;
        }
        let query = self.search_query();
        let found = if forward {
            search::find_next(&self.buffer, &query, self.cursor.row(), self.cursor.col())
        } else {
            search::find_prev(&self.buffer, &query, self.cursor.row(), self.cursor.col())
        };
        match found {
            Some(((row, col), wrapped)) => self.place_search_result(row, col, wrapped),
            None => SearchOutcome::Missing,
        }
    }
}

pub(super) fn status_line(forward: bool, pattern: &str, index: usize, total: usize) -> String {
    let delimiter = if forward { '/' } else { '?' };
    format!("{delimiter}{pattern} [{index}/{total}]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn search_backspace_empty_cancels() {
        let mut e = editor_with("foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::DeleteSearchChar);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!(e.search_prompt(), None);
    }

    #[test]
    fn search_cancel_clears_status() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        e.apply(Action::EnterSearch);
        e.apply(Action::CancelSearch);
        assert_eq!(e.search_status_text(), None);
    }

    #[test]
    fn search_cancel_restores_origin() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::MoveRight);
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::CancelSearch);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn search_confirm_shows_status() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        assert_eq!(
            e.confirm_search(),
            Some(SearchOutcome::Found { wrapped: false })
        );
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [1/1]"));
    }

    #[test]
    fn search_confirm_stays_put() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.mode(), Mode::Normal);
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn search_confirm_without_match_is_missing() {
        let mut e = editor_with("foo");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('z'));
        assert_eq!(e.confirm_search(), Some(SearchOutcome::Missing));
        assert_eq!(e.search_status_text(), None);
    }

    #[test]
    fn search_next_refreshes_status() {
        let mut e = editor_with("ar ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!((e.row(), e.col()), (0, 3));
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [2/3]"));
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [3/3]"));
        assert_eq!(e.search_prev(), SearchOutcome::Found { wrapped: false });
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [2/3]"));
    }

    #[test]
    fn search_next_reports_wrap() {
        let mut e = editor_with("ar ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
        assert_eq!((e.row(), e.col()), (0, 6));
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: true });
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn search_prev_wraps_around() {
        let mut e = editor_with("ar ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_prev(), SearchOutcome::Found { wrapped: false });
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn search_spans_clear_on_cancel() {
        let mut e = editor_with("ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_spans(0, 1).len(), 2);
        e.apply(Action::EnterSearch);
        e.apply(Action::CancelSearch);
        assert!(e.search_spans(0, 1).is_empty());
    }

    #[test]
    fn search_spans_empty_without_pattern() {
        let e = editor_with("ar ar");
        assert!(e.search_spans(0, 2).is_empty());
    }

    #[test]
    fn search_spans_follow_live_typing() {
        let mut e = editor_with("ar ar\nar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        let spans = e.search_spans(0, 2);
        assert_eq!(spans.len(), 3);
        assert!(spans[1].current);
    }

    #[test]
    fn search_spans_mark_current_match() {
        let mut e = editor_with("ar ar\nar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        let spans = e.search_spans(0, 2);
        assert_eq!(spans.len(), 3);
        assert_eq!(
            spans.iter().map(|span| span.current).collect::<Vec<_>>(),
            vec![false, true, false]
        );
        e.apply(Action::NextMatch);
        let spans = e.search_spans(0, 2);
        assert_eq!(
            spans.iter().map(|span| span.current).collect::<Vec<_>>(),
            vec![false, false, true]
        );
    }

    #[test]
    fn search_status_hides_after_edit() {
        let mut e = editor_with("ar ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        e.apply(Action::EnterInsert);
        e.apply(Action::InsertChar('x'));
        assert_eq!(e.search_status_text(), None);
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [3/3]"));
    }

    #[test]
    fn search_status_recaches_new_pattern() {
        let mut e = editor_with("ar foo ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [2/2]"));
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('f'));
        e.apply(Action::PushSearchChar('o'));
        e.apply(Action::PushSearchChar('o'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_status_text().as_deref(), Some("/foo [1/1]"));
        assert_eq!(e.search_next(), SearchOutcome::Missing);
        assert_eq!(e.search_status_text().as_deref(), Some("/foo [1/1]"));
    }

    #[test]
    fn search_typing_jumps_live() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::EnterSearch);
        assert_eq!(e.mode(), Mode::Search);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn search_without_pattern_is_missing() {
        let mut e = editor_with("foo");
        assert_eq!(e.search_next(), SearchOutcome::Missing);
        assert_eq!(e.search_prev(), SearchOutcome::Missing);
    }

    #[test]
    fn typed_word_atoms_match_whole_words() {
        let mut e = editor_with("foo foobar foo");
        e.apply(Action::EnterSearch);
        for c in "\\<foo\\>".chars() {
            e.apply(Action::PushSearchChar(c));
        }
        e.apply(Action::ConfirmSearch);
        assert_eq!((e.row(), e.col()), (0, 11));
        e.apply(Action::NextMatch);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn typed_word_atoms_still_search_literally() {
        let mut e = editor_with("a\\<b\\> a\\<b\\>");
        e.apply(Action::EnterSearch);
        for c in "a\\<b\\>".chars() {
            e.apply(Action::PushSearchChar(c));
        }
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_status_text().as_deref(), Some("/a\\<b\\> [2/2]"));
    }

    fn type_search(editor: &mut Editor, backward: bool, pattern: &str) {
        if backward {
            editor.apply(Action::EnterSearchBackward);
        } else {
            editor.apply(Action::EnterSearch);
        }
        for c in pattern.chars() {
            editor.apply(Action::PushSearchChar(c));
        }
    }

    #[test]
    fn backward_prompt_uses_question_delimiter() {
        let mut e = editor_with("foo x foo");
        e.apply(Action::EnterSearchBackward);
        assert_eq!(e.search_prompt_delimiter(), '?');
        e.apply(Action::CancelSearch);
        e.apply(Action::EnterSearch);
        assert_eq!(e.search_prompt_delimiter(), '/');
    }

    #[test]
    fn backward_typing_jumps_to_previous_match() {
        let mut e = editor_with("foo x foo");
        e.apply(Action::JumpTo(0, 6));
        type_search(&mut e, true, "foo");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn backward_typing_wraps_to_last_match() {
        let mut e = editor_with("foo x foo");
        type_search(&mut e, true, "foo");
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn backward_confirm_sets_direction_and_delimiter() {
        let mut e = editor_with("foo x foo");
        e.apply(Action::JumpTo(0, 6));
        type_search(&mut e, true, "foo");
        e.apply(Action::ConfirmSearch);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.search_status_text().as_deref(), Some("?foo [1/2]"));
        e.apply(Action::NextMatch);
        assert_eq!((e.row(), e.col()), (0, 6));
        e.apply(Action::PrevMatch);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn empty_backward_search_repeats_last_pattern() {
        let mut e = editor_with("foo x foo");
        type_search(&mut e, false, "foo");
        e.apply(Action::ConfirmSearch);
        assert_eq!((e.row(), e.col()), (0, 6));
        e.apply(Action::EnterSearchBackward);
        e.apply(Action::ConfirmSearch);
        assert_eq!((e.row(), e.col()), (0, 0));
        assert_eq!(e.search_pattern(), "foo");
        assert_eq!(e.search_status_text().as_deref(), Some("?foo [1/2]"));
    }

    #[test]
    fn empty_search_without_pattern_is_missing() {
        let mut e = editor_with("foo x foo");
        e.apply(Action::JumpTo(0, 6));
        e.apply(Action::EnterSearchBackward);
        assert_eq!(e.confirm_search(), Some(SearchOutcome::Missing));
        assert_eq!(e.search_pattern(), "");
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn cancelled_search_keeps_repeat_pattern() {
        let mut e = editor_with("foo x foo");
        e.apply(Action::JumpTo(0, 6));
        type_search(&mut e, true, "foo");
        e.apply(Action::CancelSearch);
        assert_eq!((e.row(), e.col()), (0, 6));
        e.apply(Action::EnterSearchBackward);
        e.apply(Action::ConfirmSearch);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn failed_empty_search_restores_cursor() {
        let mut e = editor_with("foo x foo");
        e.apply(Action::JumpTo(0, 6));
        type_search(&mut e, false, "zzz");
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.confirm_search(), Some(SearchOutcome::Missing));
        e.apply(Action::EnterSearchBackward);
        assert_eq!(e.confirm_search(), Some(SearchOutcome::Missing));
        assert_eq!((e.row(), e.col()), (0, 6));
    }
}
