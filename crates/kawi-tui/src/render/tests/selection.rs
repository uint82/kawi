use super::*;
use crate::render::highlight::span_segment_at;

#[test]
fn spans_map_visible_rows_to_screen_rows() {
    let lines = lines_of("one\ntwo\nthree");
    let viewport = viewport();
    let view = SpanView {
        window: &lines,
        rows: Cow::Owned(vec![0, 2]),
        viewport: &viewport,
        gutter: 0,
        leftcol: 0,
        tabstop: 8,
    };
    assert!(span_segment_at(1, 0, 1, &view, Tint::Selection).is_none());
    let span = span_segment_at(2, 0, 1, &view, Tint::Selection).expect("visible span");
    assert_eq!(span.screen_row, 1);
    assert_eq!(span.text, "t");
}

#[test]
fn no_selection_has_no_spans() {
    let lines = lines_of("abcdef");
    assert!(spans_of(&lines, 0, None, &viewport(), None).is_empty());
}

#[test]
fn point_selection_paints_one_cell_without_cursor() {
    let lines = lines_of("abcdef");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 2),
            end: (0, 2),
        }),
        &viewport(),
        None,
    );
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].from, 2);
    assert_eq!(spans[0].text, "c");
}

#[test]
fn point_selection_skips_cursor_cell() {
    let lines = lines_of("abcdef");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 2),
            end: (0, 2),
        }),
        &viewport(),
        Some((0, 2)),
    );
    assert!(spans.is_empty());
}

#[test]
fn linewise_selection_skips_cursor_cell() {
    let lines = lines_of("ab\ncd");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 0),
            end: (1, 2),
        }),
        &viewport(),
        Some((0, 0)),
    );
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].from, 1);
    assert_eq!(spans[0].text, "b");
    assert_eq!(spans[1].from, 0);
    assert_eq!(spans[1].text, "cd");
}

#[test]
fn linewise_selection_has_no_eol_cell() {
    let lines = lines_of("ab\ncd");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 0),
            end: (1, 2),
        }),
        &viewport(),
        Some((1, 0)),
    );
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "ab");
    assert_eq!(spans[1].text, "d");
}

#[test]
fn split_selection_around_cursor_cell() {
    let lines = lines_of("abcdef");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 0),
            end: (0, 6),
        }),
        &viewport(),
        Some((0, 2)),
    );
    assert_eq!(spans.len(), 2);
    assert_eq!((spans[0].from, spans[0].text.as_str()), (0, "ab"));
    assert_eq!((spans[1].from, spans[1].text.as_str()), (3, "def"));
}

#[test]
fn empty_line_inside_selection_paints_one_cell() {
    let lines = lines_of("ab\n\ncd");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 0),
            end: (2, 2),
        }),
        &viewport(),
        Some((2, 0)),
    );
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].text, "ab");
    assert_eq!((spans[1].from, spans[1].text.as_str()), (0, " "));
    assert_eq!(spans[2].text, "d");
}

#[test]
fn empty_cursor_line_paints_nothing() {
    let lines = lines_of("ab\n\ncd");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 0),
            end: (2, 0),
        }),
        &viewport(),
        Some((2, 0)),
    );
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "ab");
    assert_eq!(spans[1].text, " ");
}

#[test]
fn charwise_selection_has_no_eol_cell() {
    let lines = lines_of("abcdef\nghij");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 0),
            end: (1, 2),
        }),
        &viewport(),
        Some((1, 2)),
    );
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "abcdef");
    assert_eq!(spans[1].text, "gh");
}

#[test]
fn cursor_past_line_end_adds_no_cell() {
    let lines = lines_of("abc");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 0),
            end: (0, 3),
        }),
        &viewport(),
        Some((0, 3)),
    );
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].text, "abc");
}

#[test]
fn range_selection_spans_chars() {
    let lines = lines_of("abcdef");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 1),
            end: (0, 3),
        }),
        &viewport(),
        None,
    );
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].screen_row, 0);
    assert_eq!(spans[0].from, 1);
    assert_eq!(spans[0].text, "bcd");
}

#[test]
fn multiline_selection_spans_rows() {
    let lines = lines_of("ab\ncd\nef");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Range {
            start: (0, 1),
            end: (2, 1),
        }),
        &viewport(),
        Some((2, 1)),
    );
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].text, "b");
    assert_eq!(spans[1].text, "cd");
    assert_eq!(spans[2].text, "e");
}

#[test]
fn block_selection_skips_cursor_cell() {
    let lines = lines_of("abcd\nefgh");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Block {
            top: 0,
            left: 0,
            bottom: 1,
            right: 2,
            ragged: false,
        }),
        &viewport(),
        Some((1, 1)),
    );
    assert_eq!(spans.len(), 3);
    assert_eq!(spans[0].text, "abc");
    assert_eq!((spans[1].from, spans[1].text.as_str()), (0, "e"));
    assert_eq!((spans[2].from, spans[2].text.as_str()), (2, "g"));
}

#[test]
fn block_selection_pads_short_rows() {
    let lines = lines_of("ab\nabcd");
    let spans = spans_of(
        &lines,
        0,
        Some(Selection::Block {
            top: 0,
            left: 0,
            bottom: 1,
            right: 3,
            ragged: false,
        }),
        &viewport(),
        None,
    );
    assert_eq!(spans.len(), 2);
    assert_eq!(spans[0].text, "ab  ");
    assert_eq!(spans[1].text, "abcd");
}
