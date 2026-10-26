use crate::objects::bracket_match::{find_match, BracketMatch, MatchRequest, Position};

use super::Editor;

impl Editor {
    pub fn bracket_match(&self, bound: usize) -> Option<BracketMatch> {
        if !self.match_brackets {
            return None;
        }
        let first = self.bracket_cursor_position()?;
        let second = self.bracket_partner_at(first, bound)?;
        Some(BracketMatch { first, second })
    }

    pub(crate) fn bracket_cursor_position(&self) -> Option<Position> {
        let row = self.cursor.row();
        let col = self.cursor.col();
        if self
            .buffer
            .char_at(row, col)
            .is_some_and(|ch| self.is_match_bracket(ch))
        {
            return Some((row, col));
        }
        if self.mode.is_insert() && col > 0 {
            let before = col.saturating_sub(1);
            if self
                .buffer
                .char_at(row, before)
                .is_some_and(|ch| self.is_match_bracket(ch))
            {
                return Some((row, before));
            }
        }
        None
    }

    pub(crate) fn bracket_partner_at(&self, position: Position, bound: usize) -> Option<Position> {
        find_match(&MatchRequest {
            buffer: &self.buffer,
            position,
            language: self.language_indent.language,
            editing: &self.language_editing,
            pairs: &self.language_editing.match_pairs,
            bound,
        })
    }

    fn is_match_bracket(&self, ch: char) -> bool {
        self.language_editing
            .match_pairs
            .iter()
            .any(|pair| pair.open == ch || pair.close == ch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::Action;
    use crate::editing::Pair;
    use kawi_buffer::Buffer;

    fn editor(text: &str) -> Editor {
        Editor::new(Buffer::from_string(text))
    }

    #[test]
    fn normal_mode_matches_the_bracket_under_the_cursor() {
        let mut editor = editor("a (b) c");
        editor.apply(Action::JumpTo(0, 2));
        assert_eq!(
            editor.bracket_match(100),
            Some(BracketMatch {
                first: (0, 2),
                second: (0, 4),
            })
        );
    }

    #[test]
    fn insert_mode_falls_back_to_the_bracket_before_the_cursor() {
        let mut editor = editor("foo() bar");
        editor.apply(Action::JumpTo(0, 5));
        editor.apply(Action::EnterInsert);
        assert_eq!(
            editor.bracket_match(100),
            Some(BracketMatch {
                first: (0, 4),
                second: (0, 3),
            })
        );
    }

    #[test]
    fn insert_mode_prefers_the_bracket_under_the_cursor() {
        let mut editor = editor("foo()");
        editor.apply(Action::JumpTo(0, 3));
        editor.apply(Action::EnterInsert);
        assert_eq!(
            editor.bracket_match(100),
            Some(BracketMatch {
                first: (0, 3),
                second: (0, 4),
            })
        );
    }

    #[test]
    fn insert_mode_before_a_closer_at_line_end_matches_it() {
        let mut editor = editor("(a)");
        editor.apply(Action::JumpTo(0, 3));
        editor.apply(Action::EnterInsert);
        assert_eq!(
            editor.bracket_match(100),
            Some(BracketMatch {
                first: (0, 2),
                second: (0, 0),
            })
        );
    }

    #[test]
    fn insert_mode_without_a_bracket_returns_nothing() {
        let mut editor = editor("ab");
        editor.apply(Action::JumpTo(0, 1));
        editor.apply(Action::EnterInsert);
        assert!(editor.bracket_match(100).is_none());
    }

    #[test]
    fn disabled_highlighting_returns_nothing() {
        let mut editor = editor("(a)");
        editor.apply(Action::JumpTo(0, 0));
        editor.set_match_brackets(false);
        assert!(editor.bracket_match(100).is_none());
    }

    #[test]
    fn autopairs_do_not_control_highlighting() {
        let mut editor = editor("(a)");
        editor.apply(Action::JumpTo(0, 0));
        editor.set_auto_pairs(false);
        assert!(editor.bracket_match(100).is_some());
    }

    #[test]
    fn unmatched_bracket_returns_nothing() {
        let mut editor = editor("(a");
        editor.apply(Action::JumpTo(0, 0));
        assert!(editor.bracket_match(100).is_none());
    }

    #[test]
    fn angle_brackets_are_opt_in() {
        let mut editor = editor("<a>");
        editor.apply(Action::JumpTo(0, 0));
        assert!(editor.bracket_match(100).is_none());
        editor.set_language_editing(crate::editing::LanguageEditing {
            match_pairs: vec![Pair::new('<', '>')],
            ..crate::editing::LanguageEditing::default()
        });
        assert_eq!(
            editor.bracket_match(100),
            Some(BracketMatch {
                first: (0, 0),
                second: (0, 2),
            })
        );
    }

    #[test]
    fn visual_mode_highlights_the_bracket_under_the_cursor() {
        let mut editor = editor("(a)");
        editor.apply(Action::JumpTo(0, 0));
        editor.apply(Action::EnterVisual);
        assert_eq!(
            editor.bracket_match(100),
            Some(BracketMatch {
                first: (0, 0),
                second: (0, 2),
            })
        );
    }
}
