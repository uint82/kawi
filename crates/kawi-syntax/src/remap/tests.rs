use super::*;
use crate::registry::SyntaxHighlight;

fn span(row: usize, start: usize, end: usize) -> SyntaxSpan {
    SyntaxSpan {
        row,
        start,
        end,
        highlight: SyntaxHighlight(0),
    }
}

fn edit(start: usize, end: usize, text: &str) -> SyntaxEdit {
    SyntaxEdit {
        start,
        end,
        text: text.to_string(),
    }
}

fn shifted(spans: &[SyntaxSpan], old: &str, new: &str, edits: &[SyntaxEdit]) -> Vec<SyntaxSpan> {
    shift_spans(spans, &Rope::from_str(old), &Rope::from_str(new), edits)
}

#[test]
fn shifts_spans_after_an_insertion_before_them() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(&spans, "foo bar", "fooX bar", &[edit(3, 3, "X")]),
        vec![span(0, 5, 8)]
    );
}

#[test]
fn shifts_spans_when_text_is_inserted_at_their_start() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(&spans, "foo bar", "foo Xbar", &[edit(4, 4, "X")]),
        vec![span(0, 5, 8)]
    );
}

#[test]
fn expands_spans_when_text_is_inserted_at_their_end() {
    let spans = [span(0, 0, 3)];
    assert_eq!(
        shifted(&spans, "foo bar", "fooX bar", &[edit(3, 3, "X")]),
        vec![span(0, 0, 4)]
    );
}

#[test]
fn expands_spans_when_text_is_inserted_inside() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(&spans, "foo bar", "foo bXar", &[edit(5, 5, "X")]),
        vec![span(0, 4, 8)]
    );
}

#[test]
fn preserves_spans_when_text_inside_is_replaced() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(&spans, "foo bar", "foo bXr", &[edit(5, 6, "X")]),
        vec![span(0, 4, 7)]
    );
}

#[test]
fn keeps_spans_when_their_entire_text_is_replaced() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(&spans, "foo bar", "foo xyz", &[edit(4, 7, "xyz")]),
        vec![span(0, 4, 7)]
    );
}

#[test]
fn drops_spans_covered_by_a_larger_replacement() {
    let spans = [span(0, 4, 7)];
    assert!(shifted(&spans, "foo bar baz", "foo xyz baz", &[edit(3, 8, " xyz ")]).is_empty());
}

#[test]
fn truncates_spans_when_their_tail_is_deleted() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(&spans, "foo bar", "foo ba", &[edit(6, 7, "")]),
        vec![span(0, 4, 6)]
    );
}

#[test]
fn drops_spans_when_their_entire_text_is_deleted() {
    let spans = [span(0, 4, 7)];
    assert!(shifted(&spans, "foo bar", "foo ", &[edit(4, 7, "")]).is_empty());
}

#[test]
fn shifts_spans_after_a_deletion() {
    let spans = [span(0, 8, 13)];
    assert_eq!(
        shifted(&spans, "one two three", "one three", &[edit(3, 7, "")]),
        vec![span(0, 4, 9)]
    );
}

#[test]
fn moves_spans_between_rows() {
    let spans = [span(1, 0, 4)];
    assert_eq!(
        shifted(
            &spans,
            "alpha\nbeta\n",
            "\nalpha\nbeta\n",
            &[edit(0, 0, "\n")]
        ),
        vec![span(2, 0, 4)]
    );
}

#[test]
fn shifts_unicode_columns_by_characters() {
    let spans = [span(0, 2, 3)];
    assert_eq!(
        shifted(&spans, "é-x", "éé-x", &[edit(0, 0, "é")]),
        vec![span(0, 3, 4)]
    );
}

#[test]
fn applies_multiple_edits_in_order() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(
            &spans,
            "foo bar baz",
            "Xfoo bar bazY",
            &[edit(0, 0, "X"), edit(11, 11, "Y")]
        ),
        vec![span(0, 5, 8)]
    );
}

#[test]
fn keeps_spans_when_neighbours_join_them() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(&spans, "foo bar", "foobar", &[edit(3, 4, "")]),
        vec![span(0, 3, 6)]
    );
    assert_eq!(
        shifted(&spans, "foo bar", "foox bar", &[edit(3, 3, "x")]),
        vec![span(0, 5, 8)]
    );
    assert_eq!(
        shifted(&spans, "foo bar", "foo xbar", &[edit(4, 4, "x")]),
        vec![span(0, 5, 8)]
    );
}

#[test]
fn keeps_spans_colored_when_their_tail_changes() {
    let spans = [span(0, 0, 3)];
    assert_eq!(
        shifted(&spans, "abc def", "abX def", &[edit(2, 3, "X")]),
        vec![span(0, 0, 3)]
    );
}

#[test]
fn keeps_spans_when_a_newline_is_inserted_after_them() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(&spans, "foo bar", "foo bar\n", &[edit(7, 7, "\n")]),
        vec![span(0, 4, 7)]
    );
}

#[test]
fn shifts_spans_when_a_newline_is_inserted_before_them() {
    let spans = [span(0, 4, 7)];
    assert_eq!(
        shifted(&spans, "foo bar", "foo \nbar", &[edit(4, 4, "\n")]),
        vec![span(1, 0, 3)]
    );
}

#[test]
fn drops_spans_when_a_newline_splits_them() {
    let spans = [span(0, 0, 3)];
    assert!(shifted(&spans, "abc", "a\nbc", &[edit(1, 1, "\n")]).is_empty());
}
