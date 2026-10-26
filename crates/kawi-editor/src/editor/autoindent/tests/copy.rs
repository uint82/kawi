use super::*;

#[test]
fn enter_copies_indent_and_cursor_after_it() {
    let mut e = editor_with("    abc");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(
        e.text_lines(),
        vec!["    abc".to_string(), "    ".to_string()]
    );
    assert_eq!((e.row(), e.col()), (1, 4));
}

#[test]
fn enter_mid_text_moves_text_and_keeps_indent() {
    let mut e = editor_with("    abc");
    e.apply(Action::JumpTo(0, 5));
    e.apply(Action::EnterInsert);
    e.apply(Action::InsertNewline);
    assert_eq!(
        e.text_lines(),
        vec!["    a".to_string(), "    bc".to_string()]
    );
    assert_eq!((e.row(), e.col()), (1, 4));
}

#[test]
fn enter_inside_indent_does_not_double_indent() {
    let mut e = editor_with("    abc");
    e.apply(Action::JumpTo(0, 2));
    e.apply(Action::EnterInsert);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines(), vec!["  ".to_string(), "  abc".to_string()]);
    assert_eq!((e.row(), e.col()), (1, 2));
}

#[test]
fn enter_at_column_zero_keeps_text_unindented() {
    let mut e = editor_with("    abc");
    e.apply(Action::JumpTo(0, 0));
    e.apply(Action::EnterInsert);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines(), vec!["".to_string(), "abc".to_string()]);
}

#[test]
fn enter_uses_tabs_without_expandtab() {
    let mut e = editor_with("        abc");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(
        e.text_lines(),
        vec!["        abc".to_string(), "\t".to_string()]
    );
}

#[test]
fn enter_uses_spaces_with_expandtab() {
    let mut e = editor_with("\tabc");
    e.set_expandtab(true);
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(
        e.text_lines(),
        vec!["\tabc".to_string(), "        ".to_string()]
    );
}

#[test]
fn escape_removes_blank_autoindent() {
    let mut e = editor_with("    abc");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    e.apply(Action::ExitInsert);
    assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
    assert_eq!((e.row(), e.col()), (1, 0));
}

#[test]
fn escape_keeps_indent_when_text_typed() {
    let mut e = editor_with("    abc");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    e.apply(Action::InsertChar('x'));
    e.apply(Action::ExitInsert);
    assert_eq!(
        e.text_lines(),
        vec!["    abc".to_string(), "    x".to_string()]
    );
}

#[test]
fn escape_keeps_typed_spaces() {
    let mut e = editor_with("    abc");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    e.apply(Action::InsertChar(' '));
    e.apply(Action::ExitInsert);
    assert_eq!(
        e.text_lines(),
        vec!["    abc".to_string(), "     ".to_string()]
    );
}

#[test]
fn second_enter_clears_blank_autoindent() {
    let mut e = editor_with("    abc");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    e.apply(Action::InsertNewline);
    e.apply(Action::InsertChar('x'));
    e.apply(Action::ExitInsert);
    assert_eq!(
        e.text_lines(),
        vec!["    abc".to_string(), "".to_string(), "    x".to_string()]
    );
}

#[test]
fn backspace_in_autoindent_removes_whole_step() {
    let mut e = editor_with("    abc");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    e.apply(Action::DeleteBackward);
    assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
    assert_eq!((e.row(), e.col()), (1, 0));
}

#[test]
fn backspace_in_mid_indent_clears_to_boundary() {
    let mut e = editor_with("        abc");
    e.apply(Action::JumpTo(0, 6));
    e.apply(Action::EnterInsert);
    e.apply(Action::DeleteBackward);
    assert_eq!(e.text_lines(), vec!["  abc".to_string()]);
    assert_eq!(e.col(), 0);
}

#[test]
fn backspace_outside_indent_deletes_one_char() {
    let mut e = editor_with("    abc");
    e.apply(Action::JumpTo(0, 5));
    e.apply(Action::EnterInsert);
    e.apply(Action::DeleteBackward);
    assert_eq!(e.text_lines(), vec!["    bc".to_string()]);
}

#[test]
fn noautoindent_enter_starts_at_column_zero() {
    let mut e = editor_with("    abc");
    e.set_autoindent(false);
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
    assert_eq!((e.row(), e.col()), (1, 0));
}

#[test]
fn open_below_uses_rebuilt_indent() {
    let mut e = editor_with("\tabc");
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines(), vec!["\tabc".to_string(), "\t".to_string()]);
    assert_eq!((e.row(), e.col()), (1, 1));
}

#[test]
fn open_below_without_autoindent_has_no_indent() {
    let mut e = editor_with("    abc");
    e.set_autoindent(false);
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
}

#[test]
fn escape_after_open_removes_indent() {
    let mut e = editor_with("    abc");
    e.apply(Action::OpenLineBelow);
    e.apply(Action::ExitInsert);
    assert_eq!(e.text_lines(), vec!["    abc".to_string(), "".to_string()]);
}
