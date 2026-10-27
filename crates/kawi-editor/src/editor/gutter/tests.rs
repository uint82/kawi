use super::*;
use crate::actions::Action;
use kawi_buffer::Buffer;

fn editor_with(text: &str) -> Editor {
    Editor::new(Buffer::from_string(text))
}

fn three_lines() -> Editor {
    editor_with("\tello world\nsecond\nthird")
}

#[test]
fn gutter_shown_by_default() {
    let e = three_lines();
    assert_eq!(e.gutter_width(), 5);
    assert_eq!(e.gutter_lines(0, 3), vec!["   1 ", "   2 ", "   3 "]);
}

#[test]
fn number_shows_padded_numbers() {
    let mut e = three_lines();
    e.apply(Action::JumpTo(1, 0));
    e.set_number(true);
    assert_eq!(e.gutter_width(), 5);
    assert_eq!(e.gutter_lines(0, 3), vec!["   1 ", "   2 ", "   3 "]);
}

#[test]
fn relative_number_shows_distance_from_cursor() {
    let mut e = three_lines();
    e.apply(Action::JumpTo(1, 0));
    e.set_number(false);
    e.set_relativenumber(true);
    assert_eq!(e.gutter_lines(0, 3), vec!["   1 ", "   0 ", "   1 "]);
}

#[test]
fn number_with_relative_number_shows_absolute_on_cursor_line() {
    let mut e = three_lines();
    e.apply(Action::JumpTo(1, 0));
    e.set_number(true);
    e.set_relativenumber(true);
    assert_eq!(e.gutter_lines(0, 3), vec!["   1 ", " 2   ", "   1 "]);
}

#[test]
fn signcolumn_shifts_cursor_and_mouse_columns() {
    let mut e = editor_with("abc\ndef");
    e.apply(Action::JumpTo(0, 0));
    e.set_signcolumn_width(1);
    assert_eq!(e.gutter_width(), 6);
    assert_eq!(e.cursor_wcol(), 6);
    assert_eq!(e.buffer_col_at_screen(0, 6), 0);
    e.set_number(false);
    e.set_relativenumber(false);
    assert_eq!(e.gutter_width(), 2);
    assert_eq!(e.buffer_col_at_screen(0, 2), 0);
}

#[test]
fn numberwidth_sets_minimal_column_width() {
    let mut e = three_lines();
    e.set_number(true);
    e.set_numberwidth(1);
    assert_eq!(e.gutter_width(), 3);
    assert_eq!(e.gutter_lines(0, 3), vec![" 1 ", " 2 ", " 3 "]);
    e.set_numberwidth(6);
    assert_eq!(e.gutter_width(), 7);
    assert_eq!(e.gutter_lines(0, 1), vec!["     1 "]);
}

#[test]
fn width_grows_with_line_count() {
    let text = (1..=12)
        .map(|n| n.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    let mut e = editor_with(&text);
    e.set_number(true);
    assert_eq!(e.gutter_width(), 5);
    assert_eq!(e.gutter_lines(9, 2), vec!["  10 ", "  11 "]);
}

#[test]
fn rows_past_last_line_are_blank() {
    let mut e = three_lines();
    e.set_number(true);
    assert_eq!(e.gutter_lines(2, 2), vec!["   3 ", "     "]);
}

#[test]
fn cursor_wcol_adds_gutter_to_tab_column() {
    let mut e = editor_with("\tello world");
    e.apply(Action::JumpTo(0, 0));
    assert_eq!(e.cursor_screen_col(), 7);
    assert_eq!(e.cursor_wcol(), 12);
    e.set_number(false);
    assert_eq!(e.cursor_wcol(), 8);
}

#[test]
fn buffer_col_at_screen_skips_gutter_and_snaps_tabs() {
    let mut e = editor_with("\tello world");
    e.set_number(true);
    assert_eq!(e.buffer_col_at_screen(0, 0), 0);
    assert_eq!(e.buffer_col_at_screen(0, 4), 0);
    assert_eq!(e.buffer_col_at_screen(0, 5), 0);
    assert_eq!(e.buffer_col_at_screen(0, 11), 0);
    assert_eq!(e.buffer_col_at_screen(0, 13), 1);
    assert_eq!(e.buffer_col_at_screen(0, 99), 10);
}
