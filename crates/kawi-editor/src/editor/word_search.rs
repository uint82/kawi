use super::{EditError, Editor};
use crate::search::SearchOutcome;
use crate::words::is_word_char;

impl Editor {
    pub fn search_word(&mut self, forward: bool) -> Result<(), EditError> {
        self.run_word_search(forward, true)
    }

    pub fn search_word_bare(&mut self, forward: bool) -> Result<(), EditError> {
        self.run_word_search(forward, false)
    }

    fn run_word_search(&mut self, forward: bool, wrap_atoms: bool) -> Result<(), EditError> {
        let count = self.take_count();
        let origin = (self.cursor.row(), self.cursor.col());
        let Some((start, word, is_word)) = self.word_near_cursor() else {
            return Err(EditError::NoString);
        };
        let pattern = word_pattern(&word, wrap_atoms && is_word);
        self.set_pcmark_at(origin);
        self.cursor.set_position(start.0, start.1);
        self.search.replace(&pattern);
        self.search_history.record(&pattern);
        self.search_no_smartcase = true;
        self.search_forward = forward;
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
        let (first, last, is_word) = word_run(&chars, start)?;
        let word: String = chars[first..last].iter().collect();
        Some(((row, first), word, is_word))
    }
}

fn word_run(chars: &[char], start: usize) -> Option<(usize, usize, bool)> {
    if let Some((first, last)) = word_run_at(chars, start) {
        return Some((first, last, true));
    }
    text_run_at(chars, start).map(|(first, last)| (first, last, false))
}

fn word_run_at(chars: &[char], start: usize) -> Option<(usize, usize)> {
    let found = (start..chars.len()).find(|index| is_word_char(chars[*index]))?;
    let first = (0..found)
        .rev()
        .find(|index| !is_word_char(chars[*index]))
        .map_or(0, |index| index + 1);
    let last = (found..chars.len())
        .find(|index| !is_word_char(chars[*index]))
        .unwrap_or(chars.len());
    Some((first, last))
}

fn text_run_at(chars: &[char], start: usize) -> Option<(usize, usize)> {
    let found = (start..chars.len()).find(|index| !chars[*index].is_whitespace())?;
    let first = (0..found)
        .rev()
        .find(|index| chars[*index].is_whitespace() || is_word_char(chars[*index]))
        .map_or(0, |index| index + 1);
    let last = (found..chars.len())
        .find(|index| chars[*index].is_whitespace())
        .unwrap_or(chars.len());
    Some((first, last))
}

fn word_pattern(word: &str, wrap_atoms: bool) -> String {
    if wrap_atoms {
        format!("\\<{word}\\>")
    } else {
        word.to_string()
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

    fn search_bare_word(editor: &mut Editor, forward: bool) {
        editor.search_word_bare(forward).expect("bare word search");
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
        assert_eq!(e.search_pattern(), "\\<foo\\>");
    }

    #[test]
    fn star_fallback_uses_punctuation_run() {
        let mut e = editor_with("x,,! ,,!");
        e.apply(Action::JumpTo(0, 3));
        search_word(&mut e, true);
        assert_eq!(e.search_pattern(), ",,!");
        assert_eq!((e.row(), e.col()), (0, 5));
    }

    #[test]
    fn hash_uses_word_atoms() {
        let mut e = editor_with("foo\nfoo");
        e.apply(Action::JumpTo(1, 0));
        search_word(&mut e, false);
        assert_eq!(e.search_pattern(), "\\<foo\\>");
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

    #[test]
    fn bare_star_matches_substrings() {
        let mut e = editor_with("foobar foo xfoobar");
        search_bare_word(&mut e, true);
        assert_eq!(e.search_pattern(), "foobar");
        assert_eq!((e.row(), e.col()), (0, 12));
    }

    #[test]
    fn bare_hash_searches_backward() {
        let mut e = editor_with("foo bar foo");
        e.apply(Action::JumpTo(0, 8));
        search_bare_word(&mut e, false);
        assert_eq!(e.search_pattern(), "foo");
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn bare_star_records_bare_pattern() {
        let mut e = editor_with("foo bar");
        search_bare_word(&mut e, true);
        assert_eq!(e.history_entries().search, vec!["foo"]);
    }

    #[test]
    fn bare_star_ignores_smartcase() {
        let mut e = editor_with("foo FOO Foo foo");
        e.set_ignorecase(true);
        e.set_smartcase(true);
        e.apply(Action::JumpTo(0, 8));
        search_bare_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (0, 12));
    }

    #[test]
    fn star_keeps_smartcase_off_for_next_match() {
        let mut e = editor_with("foo FOO Foo foo");
        e.set_ignorecase(true);
        e.set_smartcase(true);
        e.apply(Action::JumpTo(0, 8));
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (0, 12));
        e.apply(Action::NextMatch);
        assert_eq!((e.row(), e.col()), (0, 0));
    }

    #[test]
    fn star_status_uses_forward_delimiter() {
        let mut e = editor_with("foo x foo x foo");
        search_word(&mut e, true);
        assert_eq!(e.search_status_text().as_deref(), Some("/\\<foo\\> [2/3]"));
    }

    #[test]
    fn hash_status_uses_backward_delimiter() {
        let mut e = editor_with("foo\nfoo");
        e.apply(Action::JumpTo(1, 0));
        search_word(&mut e, false);
        assert_eq!(e.search_status_text().as_deref(), Some("?\\<foo\\> [1/2]"));
    }

    #[test]
    fn next_match_follows_backward_search_direction() {
        let mut e = editor_with("foo x foo x foo");
        e.apply(Action::JumpTo(0, 12));
        search_word(&mut e, false);
        assert_eq!((e.row(), e.col()), (0, 6));
        e.apply(Action::NextMatch);
        assert_eq!((e.row(), e.col()), (0, 0));
        e.apply(Action::PrevMatch);
        assert_eq!((e.row(), e.col()), (0, 6));
    }

    #[test]
    fn next_match_keeps_forward_search_direction() {
        let mut e = editor_with("foo x foo x foo");
        e.apply(Action::JumpTo(0, 0));
        search_word(&mut e, true);
        assert_eq!((e.row(), e.col()), (0, 6));
        e.apply(Action::PrevMatch);
        assert_eq!((e.row(), e.col()), (0, 0));
        e.apply(Action::NextMatch);
        assert_eq!((e.row(), e.col()), (0, 6));
    }
}
