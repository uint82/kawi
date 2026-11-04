use super::*;

#[test]
fn python_class_suffix_indents() {
    let mut e = python_editor("class Solution:");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(
        e.text_lines(),
        vec!["class Solution:".to_string(), "    ".to_string()]
    );
    assert_eq!((e.row(), e.col()), (1, 4));
}

#[test]
fn python_nested_definition_indents() {
    let mut e = python_editor("class Solution:\n    def run(self):");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "        ");
    assert_eq!((e.row(), e.col()), (2, 8));
}

#[test]
fn python_annotation_keeps_indent() {
    let mut e = python_editor("class A:\n    count: int");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "    ");
}

#[test]
fn python_colon_in_comment_does_not_indent() {
    let mut e = python_editor("if ready:\n    value = 1  # if ready:");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "    ");
}

#[test]
fn python_colon_in_string_does_not_indent() {
    let mut e = python_editor("if ready:\n    value = \":\"");
    append_last_line_end(&mut e);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[2], "    ");
}

#[test]
fn python_trailing_comment_after_suffix_indents() {
    let mut e = python_editor("if ready:  # keep going");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "    ");
}

#[test]
fn python_mid_line_before_suffix_does_not_indent() {
    let mut e = python_editor("if ready: pass");
    e.apply(Action::JumpTo(0, 8));
    e.apply(Action::EnterInsert);
    e.apply(Action::InsertNewline);
    assert_eq!(
        e.text_lines(),
        vec!["if ready".to_string(), ": pass".to_string()]
    );
}

#[test]
fn bracket_openers_add_one_level() {
    let mut e = bracket_editor("foo(bar({", LanguageId::Rust);
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "    ");
}

#[test]
fn closing_and_opening_brackets_are_judged_together() {
    let mut e = bracket_editor("} else {", LanguageId::Rust);
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "    ");
}

#[test]
fn pair_newline_does_not_double_indent() {
    let mut e = python_editor("    foo()");
    e.apply(Action::JumpTo(0, 8));
    e.apply(Action::EnterInsert);
    e.apply(Action::InsertNewline);
    assert_eq!(
        e.text_lines(),
        vec![
            "    foo(".to_string(),
            "        ".to_string(),
            "    )".to_string()
        ]
    );
    assert_eq!((e.row(), e.col()), (1, 8));
}

#[test]
fn bash_block_keyword_indents() {
    let mut e = bash_editor("if ready; then");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "    ");
}

#[test]
fn bash_parameter_expansion_does_not_indent() {
    let mut e = bash_editor("echo ${value");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "");
}

#[test]
fn json_open_bracket_indents() {
    let mut e = bracket_editor("[", LanguageId::Json);
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "    ");
}

#[test]
fn toml_table_header_keeps_indent() {
    let mut e = bracket_editor("[package]", LanguageId::Toml);
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "");
}

#[test]
fn open_line_below_uses_block_rules() {
    let mut e = python_editor("class Solution:");
    e.apply(Action::OpenLineBelow);
    assert_eq!(
        e.text_lines(),
        vec!["class Solution:".to_string(), "    ".to_string()]
    );
    assert_eq!((e.row(), e.col()), (1, 4));
}

#[test]
fn open_line_above_keeps_body_indent() {
    let mut e = python_editor("class Solution:\n    value = 1");
    e.apply(Action::MoveDown);
    e.apply(Action::OpenLineAbove);
    assert_eq!(
        e.text_lines(),
        vec![
            "class Solution:".to_string(),
            "    ".to_string(),
            "    value = 1".to_string()
        ]
    );
    assert_eq!((e.row(), e.col()), (1, 4));
}

#[test]
fn open_line_above_uses_bracket_context() {
    let mut e = bracket_editor("    foo(\n    bar", LanguageId::Rust);
    e.apply(Action::MoveDown);
    e.apply(Action::OpenLineAbove);
    assert_eq!(e.text_lines()[1], "        ");
}

#[test]
fn no_autoindent_disables_language_rules() {
    let mut e = python_editor("class Solution:");
    e.set_autoindent(false);
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "");
}

#[test]
fn python_indent_uses_tabs_when_configured() {
    let mut e = editor_with_path("class Solution:", "example.py");
    e.set_language_editing(python_editing());
    e.set_language_indent(LanguageIndent {
        language: Some(LanguageId::Python),
        rules: IndentRules::new(vec![":".to_string()], Vec::new(), true),
        ..LanguageIndent::default()
    });
    e.set_expandtab(false);
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    assert_eq!(e.text_lines()[1], "\t");
}

#[test]
fn paste_is_not_reindented() {
    let mut e = python_editor("if ready:");
    e.apply(Action::AppendLineEnd);
    e.apply(Action::InsertNewline);
    e.paste("if nested:");
    assert_eq!(e.text_lines()[1], "    if nested:");
}

#[test]
fn nested_blocks_indent_from_the_innermost_opener() {
    let mut e = bracket_editor(
        "void f() {\n    if (x) {\n        foo();\n    }\n}",
        LanguageId::Cpp,
    );
    e.apply(Action::JumpTo(1, 0));
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines()[2], "        ");
}

#[test]
fn statement_inside_nested_block_keeps_its_indent() {
    let mut e = bracket_editor(
        "void f() {\n    if (x) {\n        foo();\n    }\n}",
        LanguageId::Cpp,
    );
    e.apply(Action::JumpTo(2, 0));
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines()[3], "        ");
}

#[test]
fn declaration_inside_nested_block_keeps_its_indent() {
    let mut e = bracket_editor(
        "void f() {\n    if (x) {\n        int value = 1;\n    }\n}",
        LanguageId::Cpp,
    );
    e.apply(Action::JumpTo(2, 0));
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines()[3], "        ");
}

#[test]
fn blank_line_inside_nested_block_uses_body_indent() {
    let mut e = bracket_editor("void f() {\n    if (x) {\n\n    }\n}", LanguageId::Cpp);
    e.apply(Action::JumpTo(2, 0));
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines()[3], "        ");
}

#[test]
fn closing_brace_keeps_the_current_line_indent() {
    let mut e = bracket_editor(
        "namespace demo {\nconcept Addable = requires(T value) {\n    value + value;\n};\n}",
        LanguageId::Cpp,
    );
    e.apply(Action::JumpTo(3, 0));
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines()[4], "");
}

#[test]
fn nested_closing_bracket_keeps_the_current_line_indent() {
    let mut e = bracket_editor(
        "{\n    \"a\": [\n        {\n            \"b\": 1\n        }\n    ]\n}",
        LanguageId::Json,
    );
    e.apply(Action::JumpTo(4, 0));
    e.apply(Action::OpenLineBelow);
    assert_eq!(e.text_lines()[5], "        ");
}
