use crossterm::style::Color;

use super::*;

pub(super) fn surface(width: usize, height: usize) -> Surface {
    Surface::new(width, height, Color::White, Color::Black)
}

pub(super) fn row(surface: &Surface, y: usize) -> String {
    let mut text = String::new();
    for x in 0..surface.width() {
        let cell = surface.cell(x, y).expect("cell");
        if !cell.is_continuation() {
            text.push_str(cell.text());
        }
    }
    text
}

#[test]
fn writes_text_at_the_cursor() {
    let mut surface = surface(5, 2);
    surface.move_to(1, 1);
    surface.write_text("abc");
    assert_eq!(row(&surface, 1), " abc ");
    assert_eq!(surface.cursor(), Position { x: 4, y: 1 });
}

#[test]
fn renders_tabs_and_control_characters_like_neovim() {
    let mut surface = surface(12, 1);
    surface.write_text("a\tb\u{1b}");
    assert_eq!(row(&surface, 0), "a       b^[ ");
}

#[test]
fn renders_nonprintable_cells_like_neovim() {
    let mut surface = surface(12, 1);
    surface.write_text("a\u{202e}b");
    assert_eq!(row(&surface, 0), "a<202e>b    ");
}

#[test]
fn keeps_wide_graphemes_intact() {
    let mut surface = surface(4, 1);
    surface.write_text("中x");
    let wide = surface.cell(0, 0).expect("wide start");
    let continuation = surface.cell(1, 0).expect("wide continuation");
    assert_eq!(wide.text(), "中");
    assert_eq!(wide.width(), 2);
    assert!(continuation.is_continuation());
    assert_eq!(surface.cell(2, 0).expect("x").text(), "x");
}

#[test]
fn keeps_emoji_sequences_on_two_cells() {
    let family = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
    let mut surface = surface(4, 1);
    surface.write_text(&format!("a{family}"));
    let start = surface.cell(1, 0).expect("emoji start");
    assert_eq!(start.text(), family);
    assert_eq!(start.width(), 2);
    assert!(surface.cell(2, 0).expect("continuation").is_continuation());
    assert_eq!(surface.cursor(), Position { x: 3, y: 0 });
}

#[test]
fn keeps_keycaps_on_one_cell() {
    let mut surface = surface(6, 1);
    surface.write_text("1\u{fe0f}\u{20e3} 2\u{fe0f}\u{20e3}");
    assert_eq!(
        surface.cell(0, 0).expect("keycap").text(),
        "1\u{fe0f}\u{20e3}"
    );
    assert_eq!(surface.cell(1, 0).expect("space").text(), " ");
    assert_eq!(
        surface.cell(2, 0).expect("keycap").text(),
        "2\u{fe0f}\u{20e3}"
    );
    assert_eq!(surface.cursor(), Position { x: 3, y: 0 });
}

#[test]
fn replacing_half_a_wide_grapheme_clears_both_cells() {
    let mut surface = surface(4, 1);
    surface.write_text("中");
    surface.move_to(1, 0);
    surface.write_text("x");
    assert_eq!(row(&surface, 0), " x  ");
}

#[test]
fn clear_tail_starts_at_the_wide_grapheme() {
    let mut surface = surface(4, 1);
    surface.write_text("中x");
    surface.move_to(1, 0);
    surface.clear_tail();
    assert_eq!(row(&surface, 0), "    ");
}

#[test]
fn finds_the_span_of_a_multi_cell_glyph() {
    let mut surface = surface(8, 1);
    surface.write_text("\u{202e}x");
    assert_eq!(surface.glyph_span(0, 0), (0, 6));
    assert_eq!(surface.glyph_span(3, 0), (0, 6));
}

#[test]
fn replaces_a_continuation_cell_with_a_narrow_glyph() {
    let mut surface = surface(8, 1);
    surface.write_text("\u{202e}");
    surface.move_to(3, 0);
    surface.write_text("x");
    assert_eq!(row(&surface, 0), "   x    ");
}

#[test]
fn clear_tail_starts_at_the_multi_cell_glyph() {
    let mut surface = surface(8, 1);
    surface.write_text("\u{202e}");
    surface.move_to(3, 0);
    surface.clear_tail();
    assert_eq!(row(&surface, 0), "        ");
}

#[test]
fn appends_combining_graphemes_to_the_previous_cell() {
    let mut surface = surface(3, 1);
    surface.write_text("e\u{301}x");
    assert_eq!(row(&surface, 0), "e\u{301}x ");
    assert_eq!(surface.cell(1, 0).expect("x").text(), "x");
}

#[test]
fn refuses_wide_graphemes_that_do_not_fit() {
    let mut surface = surface(1, 1);
    surface.write_text("中");
    assert_eq!(row(&surface, 0), " ");
    assert_eq!(surface.cursor(), Position { x: 1, y: 0 });
}

#[test]
fn fit_clears_cells_and_applies_new_colors() {
    let mut surface = surface(3, 1);
    surface.write_text("abc");
    surface.fit(3, 1, Color::Red, Color::Blue);
    assert_eq!(row(&surface, 0), "   ");
    let cell = surface.cell(0, 0).expect("cell");
    assert_eq!(cell.text(), " ");
    assert_eq!(cell.style().foreground(), Color::Red);
    assert_eq!(cell.style().background(), Color::Blue);
}
