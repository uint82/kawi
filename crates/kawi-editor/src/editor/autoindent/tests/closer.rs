use super::*;

#[test]
fn closer_dedents_to_matching_opener_line() {
    let mut e = bracket_editor("    foo(\n            bar\n            ", LanguageId::Rust);
    append_last_line_end(&mut e);
    e.apply(Action::InsertChar(')'));
    assert_eq!(e.text_lines()[2], "    )");
    assert_eq!((e.row(), e.col()), (2, 5));
}

#[test]
fn closer_dedent_is_one_undo_step() {
    let mut e = bracket_editor("    foo(\n            bar\n            ", LanguageId::Rust);
    let before = e.text_lines();
    append_last_line_end(&mut e);
    e.apply(Action::InsertChar(')'));
    e.apply(Action::ExitInsert);
    e.apply(Action::Undo);
    assert_eq!(e.text_lines(), before);
}

#[test]
fn closer_does_not_dedent_inside_comment() {
    let mut e = bracket_editor("    foo(\n/*\n            ", LanguageId::Rust);
    e.set_language_editing(LanguageEditing {
        comments: CommentTokens {
            line: Some("//".to_string()),
            block: Some(BlockComment::new("/*", "*/")),
        },
        pairs: vec![Pair::new('(', ')')],
        match_pairs: Vec::new(),
        ..LanguageEditing::default()
    });
    append_last_line_end(&mut e);
    e.apply(Action::InsertChar(')'));
    assert_eq!(e.text_lines()[2], "            )");
}
