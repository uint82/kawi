use super::*;
use crate::editing::{BlockComment, CommentTokens, LanguageEditing};
use crate::objects::bracket_match::ACTION_MATCH_BOUND;
use kawi_buffer::Buffer;

fn lines_of(text: &str) -> Buffer {
    Buffer::from_string(text)
}

fn charwise(start: (usize, usize), end: (usize, usize)) -> WordSpan {
    WordSpan::Charwise { start, end }
}

fn bracket(
    buffer: &Buffer,
    row: usize,
    col: usize,
    open: char,
    close: char,
    inner: bool,
    count: usize,
) -> Option<WordSpan> {
    let pair = Pair::new(open, close);
    let editing = LanguageEditing {
        comments: CommentTokens {
            line: Some("//".to_string()),
            block: Some(BlockComment::new("/*", "*/")),
        },
        match_pairs: vec![pair],
        ..LanguageEditing::default()
    };
    bracket_range(
        &MatchRequest {
            buffer,
            position: (row, col),
            language: None,
            editing: &editing,
            pairs: &editing.match_pairs,
            bound: ACTION_MATCH_BOUND,
        },
        pair,
        inner,
        count,
    )
}

fn paren(buffer: &Buffer, row: usize, col: usize, inner: bool, count: usize) -> Option<WordSpan> {
    bracket(buffer, row, col, '(', ')', inner, count)
}

#[test]
fn inner_deletes_content() {
    let lines = lines_of("foo (bar) baz");
    assert_eq!(paren(&lines, 0, 5, true, 1), Some(charwise((0, 5), (0, 7))));
}

#[test]
fn around_deletes_brackets() {
    let lines = lines_of("foo (bar) baz");
    assert_eq!(
        paren(&lines, 0, 5, false, 1),
        Some(charwise((0, 4), (0, 8)))
    );
}

#[test]
fn inner_finds_enclosing_from_outside() {
    let lines = lines_of("foo (bar) baz");
    assert_eq!(paren(&lines, 0, 0, true, 1), Some(charwise((0, 5), (0, 7))));
}

#[test]
fn inner_on_opener_works() {
    let lines = lines_of("foo (bar) baz");
    assert_eq!(paren(&lines, 0, 4, true, 1), Some(charwise((0, 5), (0, 7))));
}

#[test]
fn inner_empty_is_empty_at_inner() {
    let lines = lines_of("foo () bar");
    assert_eq!(
        paren(&lines, 0, 5, true, 1),
        Some(WordSpan::Empty { at: (0, 5) })
    );
}

#[test]
fn around_empty_deletes_parens() {
    let lines = lines_of("foo () bar");
    assert_eq!(
        paren(&lines, 0, 5, false, 1),
        Some(charwise((0, 4), (0, 5)))
    );
}

#[test]
fn inner_selects_innermost() {
    let lines = lines_of("foo (a (b) c) bar");
    assert_eq!(paren(&lines, 0, 8, true, 1), Some(charwise((0, 8), (0, 8))));
}

#[test]
fn inner_selects_outer_from_between() {
    let lines = lines_of("foo (a (b) c) bar");
    assert_eq!(
        paren(&lines, 0, 5, true, 1),
        Some(charwise((0, 5), (0, 11)))
    );
}

#[test]
fn around_selects_inner_pair() {
    let lines = lines_of("foo (a (b) c) bar");
    assert_eq!(
        paren(&lines, 0, 8, false, 1),
        Some(charwise((0, 7), (0, 9)))
    );
}

#[test]
fn unmatched_open_is_none() {
    let lines = lines_of("foo (bar baz");
    assert_eq!(paren(&lines, 0, 5, true, 1), None);
    assert_eq!(paren(&lines, 0, 5, false, 1), None);
}

#[test]
fn close_bracket_is_none() {
    let lines = lines_of("foo (bar) baz");
    assert_eq!(paren(&lines, 0, 9, true, 1), None);
    assert_eq!(paren(&lines, 0, 9, false, 1), None);
}

#[test]
fn after_pair_is_none() {
    let lines = lines_of("foo (bar) baz");
    assert_eq!(paren(&lines, 0, 12, true, 1), None);
}

#[test]
fn inner_between_pairs_takes_second() {
    let lines = lines_of("(a) (b)");
    assert_eq!(paren(&lines, 0, 3, true, 1), Some(charwise((0, 5), (0, 5))));
}

#[test]
fn around_between_pairs_takes_second() {
    let lines = lines_of("(a) (b)");
    assert_eq!(
        paren(&lines, 0, 3, false, 1),
        Some(charwise((0, 4), (0, 6)))
    );
}

#[test]
fn inner_multiline_removes_middle_line() {
    let lines = lines_of("foo (\nbar\n) baz");
    assert_eq!(
        paren(&lines, 1, 0, true, 1),
        Some(WordSpan::Linewise { first: 1, last: 1 })
    );
}

#[test]
fn around_multiline_is_charwise() {
    let lines = lines_of("foo (\nbar\n) baz");
    assert_eq!(
        paren(&lines, 1, 0, false, 1),
        Some(charwise((0, 4), (2, 0)))
    );
}

#[test]
fn around_upgrades_over_blank_tail() {
    let lines = lines_of("(a\nb)   ");
    assert_eq!(
        paren(&lines, 0, 0, false, 1),
        Some(WordSpan::Linewise { first: 0, last: 1 })
    );
}

#[test]
fn inner_indented_closer() {
    let lines = lines_of("(a\n     )");
    assert_eq!(paren(&lines, 1, 0, true, 1), Some(charwise((0, 1), (0, 1))));
}

#[test]
fn inner_counts_outward() {
    let lines = lines_of("foo (a (b) c) bar");
    assert_eq!(
        paren(&lines, 0, 8, true, 2),
        Some(charwise((0, 5), (0, 11)))
    );
}

#[test]
fn inner_count_without_outer_is_none() {
    let lines = lines_of("(a) (b)");
    assert_eq!(paren(&lines, 0, 1, true, 2), None);
}

#[test]
fn braces_match_like_parens() {
    let lines = lines_of("foo {bar} baz");
    assert_eq!(
        bracket(&lines, 0, 5, '{', '}', true, 1),
        Some(charwise((0, 5), (0, 7)))
    );
    assert_eq!(
        bracket(&lines, 0, 5, '{', '}', false, 1),
        Some(charwise((0, 4), (0, 8)))
    );
}

#[test]
fn square_brackets_match_like_parens() {
    let lines = lines_of("foo [bar] baz");
    assert_eq!(
        bracket(&lines, 0, 5, '[', ']', true, 1),
        Some(charwise((0, 5), (0, 7)))
    );
    assert_eq!(
        bracket(&lines, 0, 5, '[', ']', false, 1),
        Some(charwise((0, 4), (0, 8)))
    );
}

#[test]
fn string_pairs_are_skipped_outside_strings() {
    let lines = lines_of("f(\"a(b)c\")");
    assert_eq!(paren(&lines, 0, 0, true, 1), Some(charwise((0, 2), (0, 8))));
}

#[test]
fn string_pairs_are_used_inside_strings() {
    let lines = lines_of("f(\"a(b)c\")");
    assert_eq!(paren(&lines, 0, 4, true, 1), Some(charwise((0, 5), (0, 5))));
}

#[test]
fn comment_pairs_are_available_outside_comments() {
    let lines = lines_of("x /* (a) */ y");
    assert_eq!(paren(&lines, 0, 0, true, 1), Some(charwise((0, 6), (0, 6))));
}

#[test]
fn comment_pairs_are_used_inside_comments() {
    let lines = lines_of("x /* (a) */ y");
    assert_eq!(paren(&lines, 0, 5, true, 1), Some(charwise((0, 6), (0, 6))));
}

#[test]
fn close_bracket_selects_its_pair() {
    let lines = lines_of("foo (bar) baz");
    assert_eq!(paren(&lines, 0, 8, true, 1), Some(charwise((0, 5), (0, 7))));
}

#[test]
fn unclosed_block_comment_does_not_match_code_brackets() {
    let lines = lines_of("/* (\n */");
    assert_eq!(paren(&lines, 0, 3, true, 1), None);
}

#[test]
fn angles_match_like_parens() {
    let lines = lines_of("a <b> c");
    assert_eq!(
        bracket(&lines, 0, 3, '<', '>', true, 1),
        Some(charwise((0, 3), (0, 3)))
    );
    assert_eq!(
        bracket(&lines, 0, 3, '<', '>', false, 1),
        Some(charwise((0, 2), (0, 4)))
    );
}
