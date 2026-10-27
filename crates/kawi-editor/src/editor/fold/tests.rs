use kawi_buffer::Buffer;

use super::*;
use crate::actions::Action;

fn editor(text: &str) -> Editor {
    let mut editor = Editor::new(Buffer::from_string(text));
    editor.set_view_size(40, 10);
    editor
}

fn indented_editor() -> Editor {
    let mut editor =
        editor("def main():\n    a = 1\n    if True:\n        b = 2\n    c = 3\nd = 4\n");
    editor.set_shiftwidth(4);
    editor.set_fold_method(FoldMethod::Indent);
    editor
}

#[test]
fn indent_folds_hide_nested_blocks() {
    let mut editor = indented_editor();
    editor.close_all_folds();
    assert_eq!(editor.visible_rows(0, 10), vec![0, 1, 5]);
    assert_eq!(editor.fold_marker(1), '+');
    assert_eq!(editor.visible_line_count(), 3);
}

#[test]
fn indent_folds_ignore_trailing_blank_lines() {
    let mut editor = editor("a\n    b\n\n    c\n\nd\n");
    editor.set_shiftwidth(4);
    editor.set_fold_method(FoldMethod::Indent);
    editor.close_all_folds();
    assert_eq!(editor.visible_rows(0, 10), vec![0, 1, 4, 5]);
}

#[test]
fn zero_shiftwidth_uses_the_tabstop() {
    let mut editor = editor("a\n        b\n        c\nd\n");
    editor.set_shiftwidth(0);
    editor.set_fold_method(FoldMethod::Indent);
    editor.close_all_folds();
    assert_eq!(editor.visible_rows(0, 10), vec![0, 1, 3]);
}

#[test]
fn tabs_count_as_a_tabstop_of_indentation() {
    let mut editor = editor("a\n\tb\n\tc\nd\n");
    editor.set_shiftwidth(8);
    editor.set_fold_method(FoldMethod::Indent);
    editor.close_all_folds();
    assert_eq!(editor.visible_rows(0, 10), vec![0, 1, 3]);
}

#[test]
fn vertical_motion_skips_closed_folds() {
    let mut editor = indented_editor();
    editor.close_all_folds();
    editor.apply(Action::MoveDown);
    assert_eq!(editor.row(), 1);
    editor.apply(Action::MoveDown);
    assert_eq!(editor.row(), 5);
}

#[test]
fn closing_a_fold_moves_the_cursor_to_its_start() {
    let mut editor = indented_editor();
    editor.apply(Action::JumpTo(3, 0));
    editor.apply(Action::FoldClose);
    assert_eq!(editor.row(), 1);
    assert_eq!(editor.cursor_screen_row(), Some(1));
}

#[test]
fn display_and_screen_rows_follow_folds() {
    let mut editor = indented_editor();
    editor.close_all_folds();
    assert_eq!(editor.display_row(5), 2);
    assert_eq!(editor.buffer_row_at_display(2), 5);
    assert_eq!(editor.buffer_row_at_screen(1), Some(1));
    assert_eq!(editor.buffer_row_at_screen(2), Some(5));
}

#[test]
fn manual_visual_fold_collapses_the_selection() {
    let mut editor = editor("one\ntwo\nthree\nfour\nfive\n");
    editor.apply(Action::JumpTo(1, 0));
    editor.apply(Action::EnterVisualLine);
    editor.apply(Action::JumpTo(3, 0));
    editor.apply(Action::FoldCreate);
    assert_eq!(editor.visible_rows(0, 10), vec![0, 1, 4]);
    assert_eq!(editor.row(), 1);
}

#[test]
fn deleting_a_manual_fold_restores_its_lines() {
    let mut editor = editor("one\ntwo\nthree\nfour\nfive\n");
    editor.apply(Action::JumpTo(1, 0));
    editor.apply(Action::EnterVisualLine);
    editor.apply(Action::JumpTo(3, 0));
    editor.apply(Action::FoldCreate);
    editor.apply(Action::JumpTo(2, 0));
    editor.apply(Action::FoldDelete);
    assert_eq!(editor.visible_rows(0, 10), vec![0, 1, 2, 3, 4]);
    assert_eq!(editor.row(), 1);
}

#[test]
fn editing_adjusts_manual_folds() {
    let mut editor = editor("one\ntwo\nthree\nfour\nfive\n");
    editor.apply(Action::JumpTo(1, 0));
    editor.apply(Action::EnterVisualLine);
    editor.apply(Action::JumpTo(3, 0));
    editor.apply(Action::FoldCreate);
    editor.apply(Action::JumpTo(2, 0));
    editor.apply(Action::DeleteLine);
    editor.close_all_folds();
    assert_eq!(editor.visible_rows(0, 10), vec![0, 1, 3]);
}

#[test]
fn closed_fold_at_end_keeps_a_visible_top() {
    let mut editor = editor("one\ntwo\nthree\n");
    editor.apply(Action::JumpTo(1, 0));
    editor.apply(Action::EnterVisualLine);
    editor.apply(Action::JumpTo(2, 0));
    editor.apply(Action::FoldCreate);
    editor.viewport.place_top(2, editor.line_count());
    assert_eq!(editor.view_top(), 1);
    assert_eq!(editor.visible_rows(2, 10), vec![1]);
    assert_eq!(editor.next_visible_row(1), 1);
}

#[test]
fn fold_column_marks_the_gutter() {
    let mut editor = indented_editor();
    editor.close_all_folds();
    let gutter = editor.gutter_lines_at(&[0, 1, 5]);
    assert_eq!(gutter, vec!["   1 ", "+  2 ", "   6 "]);
    assert_eq!(editor.gutter_width(), 5);
}

#[test]
fn fold_column_can_be_hidden() {
    let mut editor = indented_editor();
    editor.close_all_folds();
    editor.set_fold_column(0);
    assert_eq!(editor.fold_column_width(), 0);
    assert_eq!(editor.gutter_lines_at(&[0, 1]), vec!["  1 ", "  2 "]);
}
