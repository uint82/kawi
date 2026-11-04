use kawi_buffer::Buffer;

const FAMILY: &str = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
const FLAG: &str = "\u{1f1ee}\u{1f1e9}";
const SKIN: &str = "\u{1f44d}\u{1f3fd}";
const ACCENT: &str = "e\u{301}";

fn line_with(fragment: &str) -> String {
    format!("a{fragment}b")
}

#[test]
fn reports_cluster_boundaries() {
    let buffer = Buffer::from_string(&line_with(FAMILY));
    assert_eq!(buffer.line_len(0), 9);
    assert_eq!(buffer.line_grapheme_count(0), 3);
    assert_eq!(buffer.last_grapheme_col(0), 8);
    assert_eq!(buffer.prev_grapheme_col(0, 8), 1);
    assert_eq!(buffer.next_grapheme_col(0, 1), 8);
    assert_eq!(buffer.snap_col(0, 4), 1);
    assert_eq!(buffer.grapheme_span(0, 4), Some((1, 8)));
}

#[test]
fn maps_screen_columns_by_cluster_width() {
    let buffer = Buffer::from_string(&line_with(FAMILY));
    assert_eq!(buffer.screen_col(0, 1, 8), 1);
    assert_eq!(buffer.screen_col(0, 8, 8), 3);
    assert_eq!(buffer.screen_span(0, 1, 8), (1, 3));
    assert_eq!(buffer.char_col(0, 1, 8), 1);
    assert_eq!(buffer.char_col(0, 2, 8), 1);
    assert_eq!(buffer.char_col(0, 3, 8), 8);
}

#[test]
fn counts_remaining_clusters() {
    let buffer = Buffer::from_string(&line_with(FAMILY));
    assert_eq!(buffer.grapheme_count_from(0, 1), 2);
    assert_eq!(buffer.grapheme_end_after(0, 1, 1), 8);
    assert_eq!(buffer.grapheme_end_after(0, 1, 2), 9);
}

#[test]
fn deletes_entire_clusters() {
    let mut buffer = Buffer::from_string(&line_with(FLAG));
    assert_eq!(buffer.delete_backward(0, 3), (0, 1));
    assert_eq!(buffer.line_text(0), "ab");

    let mut buffer = Buffer::from_string(&line_with(SKIN));
    buffer.delete_forward(0, 1);
    assert_eq!(buffer.line_text(0), "ab");
}

#[test]
fn keeps_clusters_whole_in_ranges() {
    let buffer = Buffer::from_string(&line_with(ACCENT));
    assert_eq!(buffer.range_text(0, 1, 0, 1), ACCENT);
    assert_eq!(buffer.span_text(0, 1, 0, 2), ACCENT);
}

#[test]
fn includes_newline_when_the_range_crosses_a_line() {
    let buffer = Buffer::from_string("ab\ncd");
    assert_eq!(buffer.range_text(0, 0, 0, 2), "ab\n");
}
