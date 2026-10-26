use super::*;

#[test]
fn python_enter_after_suffix_keeps_tail_indented() {
    let mut e = python_editor("if ready: pass");
    e.apply(Action::JumpTo(0, 9));
    e.apply(Action::EnterInsert);
    e.apply(Action::InsertNewline);
    assert_eq!(
        e.text_lines(),
        vec!["if ready:".to_string(), "    pass".to_string()]
    );
}

#[test]
fn dot_repeat_recomputes_block_indent() {
    let mut e = python_editor("if a:\nif b:");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    e.apply(Action::InsertChar('x'));
    e.apply(Action::ExitInsert);
    e.apply(Action::JumpTo(1, 0));
    e.apply(Action::MoveDown);
    e.apply(Action::AppendLineEnd);
    e.apply(Action::RepeatLastChange);
    e.apply(Action::ExitInsert);
    assert_eq!(
        e.text_lines(),
        vec![
            "if a:".to_string(),
            "    x".to_string(),
            "if b:".to_string(),
            "    x".to_string(),
        ]
    );
}
