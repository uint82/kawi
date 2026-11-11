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
        match_pairs: Vec::new(),
        ..LanguageEditing::default()
    }
}

fn editor_with(text: &str, pairs: Vec<Pair>) -> Editor {
    let mut editor = Editor::new(Buffer::from_string(text));
    editor.set_language_editing(LanguageEditing { pairs, ..editing() });
    editor
}

fn editor_with_auto_close(text: &str, auto_close_before: &str) -> Editor {
    let mut policy = editing();
    policy.auto_close_before = auto_close_before.to_string();
    let mut editor = editor_with(text, policy.pairs.clone());
    editor.set_language_editing(policy);
    editor
}

fn open(editor: &mut Editor, row: usize, col: usize) {
    editor.apply(Action::JumpTo(row, col));
    editor.apply(Action::EnterInsert);
}

fn assert_undo_steps(editor: &mut Editor, expected: &[&str]) {
    for text in expected {
        editor.apply(Action::Undo);
        assert_eq!(editor.text_lines(), vec![text.to_string()]);
    }
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
fn pairs_before_expression_punctuation() {
    for (text, expected) in [
        ("get,", "get(),"),
        ("get;", "get();"),
        ("get:", "get():"),
        ("get.", "get()."),
        ("get=", "get()="),
    ] {
        let mut editor = editor_with(text, editing().pairs);
        open(&mut editor, 0, 3);
        editor.apply(Action::InsertChar('('));
        assert_eq!(editor.text_lines(), vec![expected.to_string()], "{text}");
        assert_eq!((editor.row(), editor.col()), (0, 4));
    }
}

#[test]
fn does_not_pair_before_an_operator() {
    let mut editor = editor_with("a + b", editing().pairs);
    open(&mut editor, 0, 2);
    editor.apply(Action::InsertChar('('));
    assert_eq!(editor.text_lines(), vec!["a (+ b".to_string()]);
}

#[test]
fn quotes_ignore_auto_close_punctuation() {
    let mut editor = editor_with_auto_close("get,", ",?");
    open(&mut editor, 0, 3);
    editor.apply(Action::InsertChar('"'));
    assert_eq!(editor.text_lines(), vec!["get\",".to_string()]);
}

#[test]
fn empty_auto_close_before_rejects_punctuation() {
    let mut editor = editor_with_auto_close("get,", "");
    open(&mut editor, 0, 3);
    editor.apply(Action::InsertChar('('));
    assert_eq!(editor.text_lines(), vec!["get(,".to_string()]);

    let mut editor = editor_with_auto_close("]", "");
    open(&mut editor, 0, 0);
    editor.apply(Action::InsertChar('['));
    assert_eq!(editor.text_lines(), vec!["[]]".to_string()]);
}

#[test]
fn custom_auto_close_before_allows_configured_characters() {
    let mut editor = editor_with_auto_close("value?", "?");
    open(&mut editor, 0, 5);
    editor.apply(Action::InsertChar('('));
    assert_eq!(editor.text_lines(), vec!["value()?".to_string()]);
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

#[test]
fn auto_pair_insert_breaks_undo_block() {
    let mut editor = editor_with("base", editing().pairs);
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('('));
    editor.apply(Action::InsertChar('a'));
    editor.apply(Action::ExitInsert);
    assert_undo_steps(&mut editor, &["base()", "base"]);
}

#[test]
fn auto_pair_newline_breaks_undo_block() {
    let mut editor = editor_with("base", editing().pairs);
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('('));
    editor.apply(Action::InsertNewline);
    editor.apply(Action::InsertChar('a'));
    editor.apply(Action::ExitInsert);
    assert_undo_steps(&mut editor, &["base()", "base"]);
}

#[test]
fn auto_pair_backspace_joins_following_text() {
    let mut editor = editor_with("base", editing().pairs);
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('('));
    editor.apply(Action::DeleteBackward);
    editor.apply(Action::InsertChar('x'));
    editor.apply(Action::ExitInsert);
    assert_undo_steps(&mut editor, &["base()", "base"]);
}

#[test]
fn auto_pair_overtype_joins_following_text() {
    let mut editor = editor_with("base", editing().pairs);
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('('));
    editor.apply(Action::InsertChar('x'));
    editor.apply(Action::InsertChar(')'));
    editor.apply(Action::ExitInsert);
    assert_undo_steps(&mut editor, &["base()", "base"]);
}

#[test]
fn consecutive_auto_pairs_undo_separately() {
    let mut editor = editor_with("base", editing().pairs);
    editor.apply(Action::AppendLineEnd);
    editor.apply(Action::InsertChar('('));
    editor.apply(Action::InsertChar('['));
    editor.apply(Action::InsertChar('a'));
    editor.apply(Action::ExitInsert);
    assert_undo_steps(&mut editor, &["base([])", "base()", "base"]);
}
