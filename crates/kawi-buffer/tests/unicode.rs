use kawi_buffer::unicode::{display_width, needs_terminal_reposition};
use kawi_buffer::Buffer;

const FAMILY: &str = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
const FLAG: &str = "\u{1f1ee}\u{1f1e9}";
const SKIN: &str = "\u{1f44d}\u{1f3fd}";
const ACCENT: &str = "e\u{301}";
const KEYCAP_ONE: &str = "1\u{fe0f}\u{20e3}";
const KEYCAP_TWO: &str = "2\u{fe0f}\u{20e3}";

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
fn lays_out_keycaps_like_neovim() {
    let line = format!("{KEYCAP_ONE} {KEYCAP_TWO}");
    let buffer = Buffer::from_string(&line);
    assert_eq!(buffer.line_grapheme_count(0), 3);
    assert_eq!(buffer.screen_col(0, 3, 8), 1);
    assert_eq!(buffer.screen_col(0, 4, 8), 2);
    assert_eq!(buffer.char_col(0, 1, 8), 3);
    assert_eq!(buffer.char_col(0, 2, 8), 4);
}

#[test]
fn includes_newline_when_the_range_crosses_a_line() {
    let buffer = Buffer::from_string("ab\ncd");
    assert_eq!(buffer.range_text(0, 0, 0, 2), "ab\n");
}

const EMOJI_LINES: [&str; 4] = [
    "\u{2764}\u{fe0f} \u{2764}\u{fe0f}\u{200d}\u{1f525} \u{2764}\u{fe0f}\u{200d}\u{1fa79} \u{1f494} \u{1f495} \u{1f496} \u{1f497} \u{1f498} \u{1f49d}",
    "\u{1f3f3}\u{fe0f} \u{1f3f4} \u{1f3f3}\u{fe0f}\u{200d}\u{1f308} \u{1f3f3}\u{fe0f}\u{200d}\u{26a7}\u{fe0f} \u{1f1ee}\u{1f1e9} \u{1f1ef}\u{1f1f5} \u{1f1f0}\u{1f1f7} \u{1f1fa}\u{1f1f8} \u{1f1ec}\u{1f1e7} \u{1f1e9}\u{1f1ea} \u{1f1eb}\u{1f1f7} \u{1f1e7}\u{1f1f7} \u{1f1ee}\u{1f1f3} \u{1f1e8}\u{1f1f3}",
    "\u{1f3ae} \u{1f3a7} \u{1f3b9} \u{1f3b8} \u{1f3bb} \u{1f941} \u{1f3a8} \u{1f58c}\u{fe0f} \u{1f4da} \u{1f4bb} \u{1f5a5}\u{fe0f} \u{2328}\u{fe0f} \u{1f5b1}\u{fe0f} \u{1f4f1} \u{1f50b} \u{1f50c}",
    "\u{1f680} \u{1f6f8} \u{1f30c} \u{1f30d} \u{1f30e} \u{1f30f} \u{1f315} \u{1f311} \u{2600}\u{fe0f} \u{1f308} \u{26a1} \u{1f525} \u{1f4a7} \u{2744}\u{fe0f} \u{1f30a} \u{1f30b} \u{1f332} \u{1f344} \u{1f33b}",
];

#[test]
fn matches_neovim_emoji_line_layouts() {
    let expected = [26, 41, 47, 56];
    for (line, width) in EMOJI_LINES.iter().zip(expected) {
        assert_eq!(display_width(line), width);
        for token in line.split(' ') {
            assert!(needs_terminal_reposition(token));
        }
    }
}

const NEOVIM_CASES: [(&str, &str, usize); 26] = [
    (
        "zwj_family",
        "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}",
        2,
    ),
    (
        "zwj_profession",
        "\u{1f469}\u{200d}\u{1f4bb} \u{1f9d1}\u{200d}\u{1f680} \u{1f3f3}\u{fe0f}\u{200d}\u{1f308}",
        8,
    ),
    ("skin_tone", "\u{1f44d}\u{1f3fd} \u{1f44b}\u{1f3ff}", 5),
    (
        "flag",
        "\u{1f1ee}\u{1f1e9} \u{1f1ef}\u{1f1f5} \u{1f1fa}\u{1f1f8}",
        8,
    ),
    ("keycap", "1\u{fe0f}\u{20e3} #\u{fe0f}\u{20e3}", 3),
    ("vs_text_vs_emoji", "\u{2764}\u{fe0e} \u{2764}\u{fe0f}", 4),
    (
        "scotland_tag_flag",
        "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f}",
        2,
    ),
    ("combining_decomposed", "e\u{301}", 1),
    ("combining_precomposed", "\u{e9}", 1),
    (
        "zalgo",
        "\u{e0}\u{301}\u{302}\u{303}\u{304} Z\u{338}\u{322}\u{31b}a\u{337}l\u{334}g\u{336}o\u{335}",
        7,
    ),
    ("bidi_override", "abc \u{202e}def\u{202c} ghi", 23),
    ("zero_width", "a\u{200b}b\u{200c}c\u{2060}d", 16),
    ("soft_hyphen", "co\u{ad}operate", 10),
    (
        "wide_cjk",
        "\u{65e5}\u{672c}\u{8a9e} \u{ff21}\u{ff22}\u{ff23}",
        13,
    ),
    (
        "arabic",
        "\u{645}\u{631}\u{62d}\u{628}\u{627} \u{628}\u{627}\u{644}\u{639}\u{627}\u{644}\u{645}",
        13,
    ),
    ("hebrew", "\u{5e9}\u{5dc}\u{5d5}\u{5dd}", 4),
    (
        "devanagari",
        "\u{915}\u{94d}\u{937}\u{924}\u{94d}\u{930}\u{93f}\u{92f}",
        3,
    ),
    ("thai", "\u{e17}\u{e35}\u{e48}\u{e19}\u{e35}\u{e48}", 2),
    ("hangul", "\u{d55c}\u{ad6d}\u{c5b4}", 6),
    ("homoglyph", "p\u{430}ssword", 8),
    ("astral_math", "\u{1d54f}\u{1d550}\u{1d56b}", 3),
    ("ligature", "\u{fb01} \u{fb02}", 3),
    ("nbsp", "a\u{a0}b", 3),
    ("ideographic_space", "a\u{3000}b", 4),
    ("thin_space", "a\u{2009}b", 3),
    ("line_separator", "a\u{2028}b", 3),
];

const REPLACEMENT_AND_BOM: [(&str, &str, usize); 2] = [
    ("replacement_char", "\u{fffd}", 1),
    ("bom_inside", "\u{feff}x", 7),
];

#[test]
fn matches_neovim_display_widths() {
    for (name, text, width) in NEOVIM_CASES {
        assert_eq!(display_width(text), width, "{name}");
    }
    for (name, text, width) in REPLACEMENT_AND_BOM {
        assert_eq!(display_width(text), width, "{name}");
    }
}

#[test]
fn maps_screen_columns_like_neovim() {
    for (name, text, width) in NEOVIM_CASES {
        let buffer = Buffer::from_string(text);
        assert_eq!(buffer.screen_col(0, buffer.line_len(0), 8), width, "{name}");
    }
    let buffer = Buffer::from_string("\tx");
    assert_eq!(buffer.screen_col(0, buffer.line_len(0), 8), 9);
}
