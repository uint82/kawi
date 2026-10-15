use super::{Editor, SearchStatus};
use crate::mode::Mode;
use crate::search::{self, SearchOutcome};

impl Editor {
    pub(super) fn cancel_search(&mut self) {
        if let Some((row, col)) = self.search_origin {
            self.cursor.set_position(row, col);
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
            self.search_status = None;
            return None;
        }
        self.refresh_search_status();
        if self.search_status.is_some() {
            return Some(SearchOutcome::Found { wrapped: false });
        }
        Some(SearchOutcome::Missing)
    }

    pub(super) fn delete_search_char(&mut self) {
        if self.mode != Mode::Search {
            return;
        }
        self.search.pop();
        if self.search.text().is_empty() {
            self.cancel_search();
            return;
        }
        self.jump_to_first_result();
    }

    pub(super) fn enter_search(&mut self) {
        self.search_origin = Some((self.cursor.row(), self.cursor.col()));
        self.search.clear();
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

    fn jump_to_first_result(&mut self) {
        let origin = self
            .search_origin
            .unwrap_or((self.cursor.row(), self.cursor.col()));
        if let Some((row, col)) =
            search::find_first_from(&self.buffer, self.search.text(), origin.0, origin.1)
        {
            self.cursor.set_position(row, col);
        }
    }

    fn mark_current(
        &self,
        mut span: search::MatchSpan,
        anchor: (usize, usize),
    ) -> search::MatchSpan {
        span.current = (span.row, span.start) == anchor;
        span
    }

    pub(super) fn next_match(&mut self) {
        let _ = self.search_next();
    }

    fn place_search_result(&mut self, row: usize, col: usize, wrapped: bool) -> SearchOutcome {
        self.cursor.set_position(row, col);
        self.refresh_search_status();
        SearchOutcome::Found { wrapped }
    }

    pub(super) fn prev_match(&mut self) {
        let _ = self.search_prev();
    }

    pub(super) fn push_search_char(&mut self, c: char) {
        if self.mode != Mode::Search {
            return;
        }
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
        let total = match &self.search_status {
            Some(status) if status.pattern == pattern && status.revision == revision => {
                status.total
            }
            _ => search::count_matches(&self.buffer, &pattern),
        };
        if total == 0 {
            self.search_status = None;
            return;
        }
        let index = search::match_rank(
            &self.buffer,
            &pattern,
            self.cursor.row(),
            self.cursor.col(),
            total,
        );
        self.search_status = Some(SearchStatus {
            pattern,
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

    pub fn search_spans(&self, top: usize, height: usize) -> Vec<search::MatchSpan> {
        let (pattern, anchor) = match self.highlight_target() {
            Some(target) => target,
            None => return Vec::new(),
        };
        search::collect_matches(&self.buffer, pattern, top, top.saturating_add(height))
            .into_iter()
            .map(|span| self.mark_current(span, anchor))
            .collect()
    }

    fn step_search(&mut self, forward: bool) -> SearchOutcome {
        if self.search.text().is_empty() {
            return SearchOutcome::Missing;
        }
        let found = if forward {
            search::find_next(
                &self.buffer,
                self.search.text(),
                self.cursor.row(),
                self.cursor.col(),
            )
        } else {
            search::find_prev(
                &self.buffer,
                self.search.text(),
                self.cursor.row(),
                self.cursor.col(),
            )
        };
        match found {
            Some(((row, col), wrapped)) => self.place_search_result(row, col, wrapped),
            None => SearchOutcome::Missing,
        }
    }
}

pub(super) fn status_line(pattern: &str, index: usize, total: usize) -> String {
    format!("/{pattern} [{index}/{total}]")
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
    fn search_confirm_empty_is_silent() {
        let mut e = editor_with("foo");
        e.apply(Action::EnterSearch);
        assert_eq!(e.confirm_search(), None);
        assert_eq!(e.search_status_text(), None);
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
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [1/3]"));
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [2/3]"));
        assert_eq!(e.search_prev(), SearchOutcome::Found { wrapped: false });
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [1/3]"));
    }

    #[test]
    fn search_next_reports_wrap() {
        let mut e = editor_with("ar ar ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
        assert_eq!((e.row(), e.col()), (0, 3));
        assert_eq!(e.search_next(), SearchOutcome::Found { wrapped: false });
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
        assert_eq!(e.search_prev(), SearchOutcome::Found { wrapped: true });
        assert_eq!((e.row(), e.col()), (0, 6));
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
        assert!(spans[0].current);
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
            vec![true, false, false]
        );
        e.apply(Action::NextMatch);
        let spans = e.search_spans(0, 2);
        assert_eq!(
            spans.iter().map(|span| span.current).collect::<Vec<_>>(),
            vec![false, true, false]
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
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [2/3]"));
    }

    #[test]
    fn search_status_recaches_new_pattern() {
        let mut e = editor_with("ar foo ar");
        e.apply(Action::EnterSearch);
        e.apply(Action::PushSearchChar('a'));
        e.apply(Action::PushSearchChar('r'));
        e.apply(Action::ConfirmSearch);
        assert_eq!(e.search_status_text().as_deref(), Some("/ar [1/2]"));
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
}
