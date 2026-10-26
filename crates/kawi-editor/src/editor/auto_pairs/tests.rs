use super::*;
use crate::actions::Action;
use crate::editing::{BlockComment, CommentTokens, LanguageEditing, Pair};
use kawi_buffer::Buffer;

fn editing() -> LanguageEditing {
    LanguageEditing {
        comments: CommentTokens {
            line: Some("//".to_string()),
            block: Some(BlockComment::new("/*", "*/")),
        },
        pairs: vec![
            Pair::new('(', ')'),
            Pair::new('[', ']'),
            Pair::new('{', '}'),
            Pair::new('"', '"'),
        ],
    }
}

fn editor_with(text: &str, pairs: Vec<Pair>) -> Editor {
    let mut editor = Editor::new(Buffer::from_string(text));
    editor.set_language_editing(LanguageEditing {
        comments: editing().comments,
        pairs,
    });
    editor
}

fn open(editor: &mut Editor, row: usize, col: usize) {
    editor.apply(Action::JumpTo(row, col));
    editor.apply(Action::EnterInsert);
}

#[test]
fn inserts_a_pair_at_a_safe_position() {
    let mut editor = editor_with("", editing().pairs);
    editor.apply(Action::EnterInsert);
    editor.apply(Action::InsertChar('('));
    assert_eq!(editor.text_lines(), vec!["()".to_string()]);
    assert_eq!((editor.row(), editor.col()), (0, 1));
}

#[test]
fn does_not_close_before_a_word() {
    let mut editor = editor_with("word", editing().pairs);
    open(&mut editor, 0, 0);
    editor.apply(Action::InsertChar('('));
    assert_eq!(editor.text_lines(), vec!["(word".to_string()]);
}

#[test]
fn closes_before_a_closing_bracket() {
    let mut editor = editor_with("]", editing().pairs);
    open(&mut editor, 0, 0);
    editor.apply(Action::InsertChar('['));
    assert_eq!(editor.text_lines(), vec!["[]]".to_string()]);
    assert_eq!((editor.row(), editor.col()), (0, 1));
}

#[test]
fn typing_a_closer_moves_over_it() {
    let mut editor = editor_with("()", editing().pairs);
    open(&mut editor, 0, 2);
    editor.apply(Action::InsertChar(')'));
    assert_eq!(editor.text_lines(), vec!["()".to_string()]);
    assert_eq!((editor.row(), editor.col()), (0, 2));
}

#[test]
fn quotes_do_not_pair_after_a_word() {
    let mut editor = editor_with("don", editing().pairs);
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('\''));
    assert_eq!(editor.text_lines(), vec!["don'".to_string()]);
}

#[test]
fn quotes_do_not_pair_inside_comments_or_strings() {
    let mut editor = editor_with("// ", editing().pairs);
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('"'));
    assert_eq!(editor.text_lines(), vec!["// \"".to_string()]);

    let mut editor = editor_with("\"text", editing().pairs);
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('"'));
    assert_eq!(editor.text_lines(), vec!["\"text\"".to_string()]);
}

#[test]
fn quotes_do_not_pair_in_multiline_block_comments() {
    let mut editor = editor_with("/*\ntext\n", editing().pairs);
    editor.apply(Action::JumpTo(1, 0));
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('"'));
    assert_eq!(
        editor.text_lines(),
        vec!["/*".to_string(), "text\"".to_string()]
    );
}

#[test]
fn language_table_controls_quotes() {
    let mut editor = editor_with("", vec![Pair::new('(', ')')]);
    editor.apply(Action::EnterInsert);
    editor.apply(Action::InsertChar('\''));
    assert_eq!(editor.text_lines(), vec!["'".to_string()]);
}

#[test]
fn backspace_removes_an_empty_pair() {
    let mut editor = editor_with("()", editing().pairs);
    open(&mut editor, 0, 1);
    editor.apply(Action::DeleteBackward);
    assert_eq!(editor.text_lines(), vec![String::new()]);
    assert_eq!((editor.row(), editor.col()), (0, 0));
}

#[test]
fn enter_splits_a_bracket_pair_with_indent() {
    let mut editor = editor_with("    foo()", editing().pairs);
    editor.set_shiftwidth(4);
    editor.set_expandtab(true);
    open(&mut editor, 0, 8);
    editor.apply(Action::InsertNewline);
    assert_eq!(
        editor.text_lines(),
        vec![
            "    foo(".to_string(),
            "        ".to_string(),
            "    )".to_string()
        ]
    );
    assert_eq!((editor.row(), editor.col()), (1, 8));
}

#[test]
fn pair_insertion_is_one_undo_step() {
    let mut editor = editor_with("", editing().pairs);
    editor.apply(Action::EnterInsert);
    editor.apply(Action::InsertChar('('));
    editor.apply(Action::ExitInsert);
    editor.apply(Action::Undo);
    assert_eq!(editor.text_lines(), vec![String::new()]);
}

#[test]
fn pair_insertion_repeats_with_dot() {
    let mut editor = editor_with("ab\ncd\n", editing().pairs);
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('('));
    editor.apply(Action::ExitInsert);
    editor.apply(Action::MoveDown);
    editor.apply(Action::RepeatLastChange);
    assert_eq!(
        editor.text_lines(),
        vec!["ab()".to_string(), "cd()".to_string()]
    );
}

#[test]
fn paste_bypasses_pairing() {
    let mut editor = editor_with("", editing().pairs);
    editor.apply(Action::EnterInsert);
    editor.paste("(");
    assert_eq!(editor.text_lines(), vec!["(".to_string()]);
}

#[test]
fn visual_selection_is_surrounded() {
    let mut editor = editor_with("word", editing().pairs);
    editor.apply(Action::JumpTo(0, 0));
    editor.apply(Action::EnterVisual);
    editor.apply(Action::MoveRight);
    editor.apply(Action::MoveRight);
    editor.apply(Action::MoveRight);
    editor.apply(Action::SurroundSelection { open: '(' });
    assert_eq!(editor.text_lines(), vec!["(word)".to_string()]);
    assert_eq!(editor.mode(), crate::mode::Mode::Normal);
}

#[test]
fn global_switch_disables_pairing() {
    let mut editor = editor_with("", editing().pairs);
    editor.set_auto_pairs(false);
    editor.apply(Action::EnterInsert);
    editor.apply(Action::InsertChar('('));
    assert_eq!(editor.text_lines(), vec!["(".to_string()]);
}
