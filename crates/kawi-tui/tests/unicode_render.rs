use kawi_tui::display::{screen_len, slice_screen};

const RENDERED_CASES: [(&str, &str, &str); 13] = [
    (
        "bidi_override",
        "abc \u{202e}def\u{202c} ghi",
        "abc <202e>def<202c> ghi",
    ),
    (
        "zero_width",
        "a\u{200b}b\u{200c}c\u{2060}d",
        "a<200b>b\u{200c}c<2060>d",
    ),
    ("bom_inside", "\u{feff}x", "<feff>x"),
    ("soft_hyphen", "co\u{ad}operate", "co\u{ad}operate"),
    ("escape", "a\u{1b}[31mb", "a^[[31mb"),
    ("delete", "a\u{7f}b", "a^?b"),
    ("c1_control", "a\u{85}b", "a<85>b"),
    ("leading_combining", "\u{301}a", " \u{301}a"),
    ("tab", "\tx", "        x"),
    (
        "devanagari",
        "\u{915}\u{94d}\u{937}\u{924}\u{94d}\u{930}\u{93f}\u{92f}",
        "\u{915}\u{94d}\u{937}\u{924}\u{94d}\u{930}\u{93f}\u{92f}",
    ),
    (
        "zalgo",
        "\u{e0}\u{301}\u{302}\u{303}\u{304}",
        "\u{e0}\u{301}\u{302}\u{303}\u{304}",
    ),
    (
        "vs_text_vs_emoji",
        "\u{2764}\u{fe0e} \u{2764}\u{fe0f}",
        "\u{2764}\u{fe0e} \u{2764}\u{fe0f}",
    ),
    (
        "scotland_tag_flag",
        "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f}",
        "\u{1f3f4}\u{e0067}\u{e0062}\u{e0073}\u{e0063}\u{e0074}\u{e007f}",
    ),
];

#[test]
fn renders_cells_like_neovim() {
    for (name, text, expected) in RENDERED_CASES {
        assert_eq!(slice_screen(text, 0, 80, 8), expected, "{name}");
    }
}

#[test]
fn measures_cells_like_neovim() {
    assert_eq!(screen_len("abc \u{202e}def\u{202c} ghi", 8), 23);
    assert_eq!(screen_len("a\u{200b}b\u{200c}c\u{2060}d", 8), 16);
    assert_eq!(screen_len("co\u{ad}operate", 8), 10);
    assert_eq!(
        screen_len(
            "\u{915}\u{94d}\u{937}\u{924}\u{94d}\u{930}\u{93f}\u{92f}",
            8
        ),
        3
    );
    assert_eq!(screen_len("\u{2764}\u{fe0e} \u{2764}\u{fe0f}", 8), 4);
    assert_eq!(screen_len("\tx", 8), 9);
}
