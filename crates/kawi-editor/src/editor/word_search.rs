use super::{EditError, Editor};
use crate::search::SearchOutcome;
use crate::words::is_word_char;

impl Editor {
    pub fn search_word(&mut self, forward: bool) -> Result<(), EditError> {
        let count = self.take_count();
        let origin = (self.cursor.row(), self.cursor.col());
        let Some((start, word, whole_word)) = self.word_near_cursor() else {
            return Err(EditError::NoString);
        };
        self.set_pcmark_at(origin);
        self.cursor.set_position(start.0, start.1);
        self.search.replace(&word);
        self.search_history.record(&word);
        self.search_whole_word = whole_word;
        self.search_origin = Some(origin);
        self.search_status = None;
        for _ in 0..count.max(1) {
            let outcome = if forward {
                self.search_next()
            } else {
                self.search_prev()
            };
            if matches!(outcome, SearchOutcome::Missing) {
                break;
            }
        }
        Ok(())
    }

    fn word_near_cursor(&self) -> Option<((usize, usize), String, bool)> {
        let row = self.cursor.row();
        let line = self.buffer.line(row)?;
        let chars: Vec<char> = line.chars().collect();
        let start = self.cursor.col().min(chars.len());
        if let Some(found) = (start..chars.len()).find(|index| is_word_char(chars[*index])) {
            let first = (0..found)
                .rev()
                .find(|index| !is_word_char(chars[*index]))
                .map_or(0, |index| index + 1);
            let last = (found..chars.len())
                .find(|index| !is_word_char(chars[*index]))
                .unwrap_or(chars.len());
            let word: String = chars[first..last].iter().collect();
            return Some(((row, first), word, true));
        }
        let found = (start..chars.len()).find(|index| !chars[*index].is_whitespace())?;
        let first = (0..found)
            .rev()
            .find(|index| chars[*index].is_whitespace())
            .map_or(0, |index| index + 1);
        let last = (found..chars.len())
            .find(|index| chars[*index].is_whitespace())
            .unwrap_or(chars.len());
        let word: String = chars[first..last].iter().collect();
        Some(((row, first), word, false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use kawi_buffer::Buffer;

    fn editor_with(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    fn search_word(editor: &mut Editor, forward: bool) {
        editor.search_word(forward).expect("word search");
    }

    #[test]
    fn star_searches_next_whole_word() {
        let mut e = editor_with("foo bar foo foobar\nfoo");
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (0, 8));
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn star_from_mid_word_uses_whole_word() {
        let mut e = editor_with("abcd abcd abcd");
        e.apply(Action::JumpTo(0, 2));
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn star_on_punctuation_finds_next_word() {
        let mut e = editor_with("a, b, a");
        e.apply(Action::JumpTo(0, 5));
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn star_wraps_around() {
        let mut e = editor_with("foo bar\nfoo bar");
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (1, 0));
    }

    #[test]
    fn hash_searches_backward() {
        let mut e = editor_with("foo bar\nfoo bar\nfoo bar");
        e.apply(Action::JumpTo(2, 4));
        search_word(&mut e, false);
        assert_eq!((e.row(), e.col()), (1, 4));
    }

    #[test]
    fn star_on_empty_line_reports_no_string() {
        let mut e = editor_with("");
        assert_eq!(e.search_word(true), Err(EditError::NoString));
    }

    #[test]
    fn star_sets_search_pattern() {
        let mut e = editor_with("foo bar foo");
        search_word(&mut e, true);
        assert_eq!(e.search_pattern(), "foo");
    }

    #[test]
    fn star_respects_ignorecase() {
        let mut e = editor_with("foo FOO Foo foo");
        e.set_ignorecase(true);
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (0, 4));
    }

    #[test]
    fn star_ignores_smartcase() {
        let mut e = editor_with("foo FOO Foo foo");
        e.set_ignorecase(true);
        e.set_smartcase(true);
        e.apply(Action::JumpTo(0, 4));
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (0, 8));
    }

    #[test]
    fn star_count_searches_nth_occurrence() {
        let mut e = editor_with("foo x foo\nfoo x foo\nfoo x foo");
        e.apply(Action::JumpTo(1, 2));
        e.apply(Action::PushCount(2));
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (2, 0));
    }
}
