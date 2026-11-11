use crate::editor::lexical::{LexicalContext, LexicalScanner};

use super::{
    context_start, is_escaped, line_chars, MatchRequest, Position, LEXICAL_CONTEXT_PADDING,
};

pub fn find_open_context(request: &MatchRequest<'_>, opens: &[char]) -> Option<Position> {
    if opens.is_empty() || request.bound == 0 {
        return None;
    }
    let (row, col) = request.position;
    let start_row = context_start(
        request.buffer,
        row,
        col,
        request.bound.saturating_add(LEXICAL_CONTEXT_PADDING),
    );
    let mut scanner = LexicalScanner::new(LexicalContext::with_language(
        request.editing,
        request.language,
    ));
    let mut stack: Vec<(Position, char)> = Vec::new();
    for current_row in start_row..=row {
        let chars = line_chars(request.buffer, current_row)?;
        let end = if current_row == row {
            col.min(chars.len())
        } else {
            chars.len()
        };
        for index in 0..end {
            let code = scanner.in_code();
            if code && !is_escaped(&chars, index) {
                track_bracket(request, current_row, index, &chars[index], &mut stack);
            }
            scanner.advance(&chars, index);
        }
        if current_row < row {
            scanner.finish_line();
        }
    }
    if !scanner.in_code() {
        return None;
    }
    stack
        .iter()
        .rev()
        .find(|(_, open)| opens.contains(open))
        .map(|(position, _)| *position)
}

fn track_bracket(
    request: &MatchRequest<'_>,
    row: usize,
    index: usize,
    character: &char,
    stack: &mut Vec<(Position, char)>,
) {
    if request
        .pairs
        .iter()
        .any(|pair| pair.open == pair.close && pair.open == *character)
    {
        return;
    }
    if let Some(pair) = request.pairs.iter().find(|pair| pair.open == *character) {
        stack.push(((row, index), pair.open));
        return;
    }
    if let Some(pair) = request.pairs.iter().find(|pair| pair.close == *character) {
        if let Some(position) = stack.iter().rposition(|(_, open)| *open == pair.open) {
            stack.truncate(position);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editing::{default_match_pairs, default_pairs, CommentTokens, LanguageEditing};
    use kawi_buffer::Buffer;

    const OPENS: [char; 3] = ['(', '[', '{'];

    fn editing() -> LanguageEditing {
        LanguageEditing {
            comments: CommentTokens {
                line: Some("//".to_string()),
                block: None,
            },
            pairs: default_pairs(),
            match_pairs: default_match_pairs(),
            ..LanguageEditing::default()
        }
    }

    fn context_with(
        text: &str,
        row: usize,
        col: usize,
        bound: usize,
        opens: &[char],
    ) -> Option<Position> {
        let buffer = Buffer::from_string(text);
        let editing = editing();
        find_open_context(
            &MatchRequest {
                buffer: &buffer,
                position: (row, col),
                language: None,
                editing: &editing,
                pairs: &editing.match_pairs,
                bound,
            },
            opens,
        )
    }

    fn context(text: &str, row: usize, col: usize) -> Option<Position> {
        context_with(text, row, col, 4_096, &OPENS)
    }

    #[test]
    fn finds_an_unclosed_open_bracket() {
        assert_eq!(context("call(", 0, 5), Some((0, 4)));
    }

    #[test]
    fn ignores_a_balanced_pair() {
        assert_eq!(context("call()", 0, 6), None);
    }

    #[test]
    fn reports_the_outer_call_before_the_inner_one_closes() {
        assert_eq!(context("call(inner()", 0, 12), Some((0, 4)));
    }

    #[test]
    fn ignores_brackets_inside_strings_and_comments() {
        assert_eq!(context("call(\"(\"", 0, 8), Some((0, 4)));
        assert_eq!(context("// (\ncall", 1, 4), None);
    }

    #[test]
    fn ignores_escaped_brackets() {
        assert_eq!(context("call(\\(", 0, 7), Some((0, 4)));
    }

    #[test]
    fn an_empty_open_list_matches_nothing() {
        assert_eq!(context_with("call(", 0, 5, 4_096, &[]), None);
    }

    #[test]
    fn a_zero_bound_matches_nothing() {
        assert_eq!(context_with("call(", 0, 5, 0, &OPENS), None);
    }

    #[test]
    fn brackets_before_the_cursor_only_are_tracked() {
        assert_eq!(context("call(", 0, 4), None);
        assert_eq!(context("call(", 0, 5), Some((0, 4)));
    }

    #[test]
    fn multiline_calls_keep_the_open_bracket() {
        assert_eq!(context("call(\n    value", 1, 9), Some((0, 4)));
    }

    #[test]
    fn string_brackets_do_not_close_the_call() {
        assert_eq!(context("call(\")\")", 0, 8), Some((0, 4)));
        assert_eq!(context("call(\")\")", 0, 9), None);
    }
}
