use kawi_buffer::Buffer;
use kawi_editor::{Action, Editor};

const FAMILY: &str = "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}\u{200d}\u{1f466}";
const FLAG: &str = "\u{1f1ee}\u{1f1e9}";
const SKIN: &str = "\u{1f44d}\u{1f3fd}";
const ACCENT: &str = "e\u{301}";

fn editor_with(text: &str) -> Editor {
    Editor::new(Buffer::from_string(text))
}

fn line_with(fragment: &str) -> String {
    format!("a{fragment}b")
}

#[test]
fn move_right_steps_over_clusters() {
    let mut editor = editor_with(&line_with(FAMILY));
    editor.apply(Action::MoveRight);
    assert_eq!(editor.col(), 1);
    editor.apply(Action::MoveRight);
    assert_eq!(editor.col(), 8);
    editor.apply(Action::MoveRight);
    assert_eq!(editor.col(), 8);
}

#[test]
fn move_left_steps_over_clusters() {
    let mut editor = editor_with(&line_with(FLAG));
    editor.apply(Action::MoveRight);
    editor.apply(Action::MoveRight);
    assert_eq!(editor.col(), 3);
    editor.apply(Action::MoveLeft);
    assert_eq!(editor.col(), 1);
    editor.apply(Action::MoveLeft);
    assert_eq!(editor.col(), 0);
}

#[test]
fn append_starts_after_the_cluster() {
    let mut editor = editor_with(&line_with(SKIN));
    editor.apply(Action::MoveRight);
    editor.apply(Action::Append);
    assert_eq!(editor.col(), 3);
    editor.apply(Action::InsertChar('X'));
    assert_eq!(editor.text_lines()[0], format!("a{SKIN}Xb"));
}

#[test]
fn delete_forward_removes_the_cluster() {
    let mut editor = editor_with(&line_with(FAMILY));
    editor.apply(Action::MoveRight);
    editor.apply(Action::DeleteForward);
    assert_eq!(editor.text_lines()[0], "ab");
    assert_eq!(editor.col(), 1);
}

#[test]
fn delete_backward_removes_the_cluster() {
    let mut editor = editor_with(&line_with(ACCENT));
    editor.apply(Action::MoveRight);
    editor.apply(Action::MoveRight);
    editor.apply(Action::EnterInsert);
    editor.apply(Action::DeleteBackward);
    assert_eq!(editor.text_lines()[0], "ab");
    assert_eq!(editor.col(), 1);
}

#[test]
fn replace_char_replaces_the_cluster() {
    let mut editor = editor_with(&line_with(FAMILY));
    editor.apply(Action::MoveRight);
    editor.apply(Action::ReplaceChar('X'));
    assert_eq!(editor.text_lines()[0], "aXb");
    assert_eq!(editor.col(), 1);
}

#[test]
fn toggle_char_keeps_combining_marks() {
    let mut editor = editor_with(&line_with(ACCENT));
    editor.apply(Action::MoveRight);
    editor.apply(Action::ToggleChar);
    assert_eq!(editor.text_lines()[0], "aE\u{301}b");
    assert_eq!(editor.col(), 3);
}

#[test]
fn line_end_lands_on_the_last_cluster_start() {
    let mut editor = editor_with(&format!("a{FAMILY}"));
    editor.apply(Action::MoveToLineEnd);
    assert_eq!(editor.col(), 1);
}

#[test]
fn visual_delete_keeps_clusters_whole() {
    let mut editor = editor_with(&line_with(FAMILY));
    editor.apply(Action::MoveRight);
    editor.apply(Action::EnterVisual);
    editor.apply(Action::DeleteSelection);
    assert_eq!(editor.text_lines()[0], "ab");
    assert_eq!(editor.col(), 1);
}

#[test]
fn screen_columns_follow_cluster_width() {
    let mut editor = editor_with(&line_with(FAMILY));
    assert_eq!(editor.cursor_screen_col(), 0);
    editor.apply(Action::MoveRight);
    assert_eq!(editor.cursor_screen_col(), 1);
    editor.apply(Action::MoveRight);
    assert_eq!(editor.cursor_screen_col(), 3);
}

#[test]
fn word_motion_does_not_split_combining_marks() {
    let mut editor = editor_with(&format!("{ACCENT} foo"));
    editor.apply(Action::MoveToNextWordStart);
    assert_eq!(editor.col(), 3);
}

#[test]
fn replace_mode_restores_the_whole_cluster() {
    let mut editor = editor_with(&line_with(FAMILY));
    editor.apply(Action::JumpTo(0, 1));
    editor.apply(Action::EnterReplace);
    editor.apply(Action::InsertChar('X'));
    assert_eq!(editor.text_lines()[0], "aXb");
    editor.apply(Action::DeleteBackward);
    assert_eq!(editor.text_lines()[0], line_with(FAMILY));
    assert_eq!(editor.col(), 1);
}

#[test]
fn word_motion_treats_emoji_as_one_cluster() {
    let mut editor = editor_with("foo \u{1f600} bar");
    editor.apply(Action::MoveToNextWordStart);
    assert_eq!(editor.col(), 4);
    editor.apply(Action::MoveToNextWordStart);
    assert_eq!(editor.col(), 6);
    editor.apply(Action::MoveToNextWordStart);
    assert_eq!(editor.col(), 8);
    editor.apply(Action::MoveToNextWordStart);
    assert_eq!(editor.col(), 8);
}

#[test]
fn word_motion_keeps_family_clusters_whole() {
    let mut editor = editor_with(&format!("a{FAMILY}b c"));
    editor.apply(Action::MoveToNextWordStart);
    assert_eq!(editor.col(), 1);
    editor.apply(Action::MoveToNextWordStart);
    assert_eq!(editor.col(), 8);
    editor.apply(Action::MoveToNextWordStart);
    assert_eq!(editor.col(), 10);
}

#[test]
fn paste_after_cluster_inserts_after_it() {
    let mut editor = editor_with(&format!("a{FAMILY}b\nZ"));
    editor.apply(Action::MoveDown);
    editor.apply(Action::MoveToLineStart);
    editor.apply(Action::YankToEnd);
    editor.apply(Action::MoveUp);
    editor.apply(Action::MoveToLineStart);
    editor.apply(Action::MoveRight);
    editor.apply(Action::PasteAfter);
    assert_eq!(editor.text_lines()[0], format!("a{FAMILY}Zb"));
    assert_eq!(editor.col(), 8);
}

#[test]
fn delete_to_end_removes_trailing_cluster() {
    let mut editor = editor_with(&format!("a{FAMILY}"));
    editor.apply(Action::MoveToLineEnd);
    editor.apply(Action::DeleteToEnd);
    assert_eq!(editor.text_lines()[0], "a");
    assert_eq!(editor.col(), 0);
}
