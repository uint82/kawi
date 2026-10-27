use super::super::text;

#[test]
fn strips_csi_osc_and_character_escapes() {
    assert_eq!(
        text::sanitize("a\u{1b}[31mb\u{1b}]0;title\u{7}c\u{1b}(0d"),
        "abcd"
    );
}

#[test]
fn strips_c1_sequences() {
    assert_eq!(text::sanitize("\u{9b}31mX\u{9d}0;title\u{9c}Y"), "XY");
}

#[test]
fn replaces_remaining_controls_with_spaces() {
    assert_eq!(text::sanitize("a\tb\nc\u{7f}"), "a b c ");
}

#[test]
fn truncates_left_on_grapheme_boundaries() {
    assert_eq!(text::truncate_left("a\u{1f642}b", 2), "\u{2026}b");
    assert_eq!(text::truncate_left("\u{1f642}\u{1f642}", 2), "\u{2026}");
    assert_eq!(text::truncate_left("abc", 1), "\u{2026}");
}

#[test]
fn truncates_right_without_splitting_wide_characters() {
    assert_eq!(text::truncate_right("\u{4e2d}\u{4e2d}", 3), "\u{4e2d} ");
    assert_eq!(text::truncate_right("abc", 3), "abc");
}

#[test]
fn measures_whole_string_width() {
    assert_eq!(text::display_width("\u{4e2d}\u{6587}"), 4);
    assert_eq!(text::display_width("\u{1f642}"), 2);
    assert_eq!(text::display_width("e\u{301}"), 1);
}
