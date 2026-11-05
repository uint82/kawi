use kawi_buffer::Buffer;

#[test]
fn keeps_unicode_line_separators_inside_a_line() {
    let buffer = Buffer::from_string("a\u{2028}b\u{2029}c");
    assert_eq!(buffer.line_count(), 1);
    assert_eq!(buffer.line(0).as_deref(), Some("a\u{2028}b\u{2029}c"));
    assert_eq!(buffer.screen_col(0, buffer.line_len(0), 8), 5);
}
