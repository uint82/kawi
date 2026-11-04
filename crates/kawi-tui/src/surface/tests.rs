use crossterm::style::Color;

use super::*;

fn surface(width: usize, height: usize) -> Surface {
    Surface::new(width, height, Color::White, Color::Black)
}

fn row(surface: &Surface, y: usize) -> String {
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
fn expands_tabs_and_sanitizes_controls() {
    let mut surface = surface(10, 1);
    surface.write_text("a\tb\u{1b}");
    assert_eq!(row(&surface, 0), "a       b ");
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
