use super::*;
use crate::actions::Action;
use crate::editing::{BlockComment, CommentTokens, LanguageEditing, Pair};
use kawi_buffer::Buffer;
use kawi_syntax::LanguageId;

use crate::BufferOptionDefaults;

use super::super::indent::IndentRules;
use super::super::options::LanguageIndent;

fn editor_with(text: &str) -> Editor {
    Editor::new(Buffer::from_string(text))
}

fn language_editor(text: &str, language: LanguageId, rules: IndentRules) -> Editor {
    language_editor_with_width(text, language, rules, 4)
}

fn language_editor_with_width(
    text: &str,
    language: LanguageId,
    rules: IndentRules,
    shiftwidth: usize,
) -> Editor {
    let mut editor = editor_with(text);
    editor.set_buffer_defaults(BufferOptionDefaults {
        shiftwidth: Some(shiftwidth),
        expandtab: Some(true),
        autoindent: true,
    });
    editor.set_language_indent(LanguageIndent {
        language: Some(language),
        rules,
        ..LanguageIndent::default()
    });
    editor
}

fn editor_with_path(text: &str, path: &str) -> Editor {
    let mut buffer = Buffer::from_string(text);
    buffer.path = Some(path.to_string());
    Editor::new(buffer)
}

fn python_editor(text: &str) -> Editor {
    let mut editor = language_editor(
        text,
        LanguageId::Python,
        IndentRules::new(vec![":".to_string()], Vec::new(), true),
    );
    editor.set_language_editing(python_editing());
    editor
}

fn bash_editor(text: &str) -> Editor {
    language_editor(
        text,
        LanguageId::Bash,
        IndentRules::new(
            Vec::new(),
            vec!["then".to_string(), "do".to_string(), "{".to_string()],
            true,
        ),
    )
}

fn bracket_editor(text: &str, language: LanguageId) -> Editor {
    language_editor(text, language, IndentRules::default())
}

fn html_editor(text: &str) -> Editor {
    let mut editor = language_editor(
        text,
        LanguageId::Html,
        IndentRules::default().with_indent_tags(true),
    );
    editor.set_language_editing(html_editing());
    editor
}

fn append_last_line_end(editor: &mut Editor) {
    let row = editor.text_lines().len().saturating_sub(1);
    let col = editor.text_lines()[row].chars().count();
    editor.apply(Action::JumpTo(row, col));
    editor.apply(Action::AppendLineEnd);
}

fn sql_editor(text: &str) -> Editor {
    let mut editor = language_editor_with_width(text, LanguageId::Sql, sql_rules(), 2);
    editor.set_language_editing(sql_editing());
    editor
}

fn sql_rules() -> IndentRules {
    IndentRules::new(
        Vec::new(),
        words(&[
            "select",
            "from",
            "where",
            "by",
            "on",
            "set",
            "values",
            "case",
            "when",
            "then",
            "else",
            "begin",
            "do",
            "repeat",
            "exception",
        ]),
        true,
    )
    .with_indent_block_openers([
        "begin",
        "case",
        "if",
        "for",
        "while",
        "loop",
        "repeat",
        "exception",
    ])
    .with_indent_block_closers(["end"])
    .with_indent_align_keywords([
        "select",
        "from",
        "where",
        "insert",
        "set",
        "values",
        "join",
        "exception",
        "until",
    ])
    .with_indent_branch_keywords(["when", "else"])
    .with_indent_branch_openers(["case", "exception"])
    .with_indent_ignore_case(true)
    .with_indent_keywords_in_brackets(true)
}

fn words(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

fn sql_editing() -> LanguageEditing {
    LanguageEditing {
        comments: CommentTokens {
            line: Some("--".to_string()),
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

fn type_text(editor: &mut Editor, text: &str) {
    for ch in text.chars() {
        editor.apply(Action::InsertChar(ch));
    }
}

fn html_editing() -> LanguageEditing {
    LanguageEditing {
        comments: CommentTokens {
            line: None,
            block: Some(BlockComment::new("<!--", "-->")),
        },
        pairs: vec![
            Pair::new('(', ')'),
            Pair::new('[', ']'),
            Pair::new('{', '}'),
            Pair::new('<', '>'),
            Pair::new('\'', '\''),
            Pair::new('"', '"'),
        ],
        match_pairs: Vec::new(),
        ..LanguageEditing::default()
    }
}

fn python_editing() -> LanguageEditing {
    LanguageEditing {
        comments: CommentTokens {
            line: Some("#".to_string()),
            block: None,
        },
        pairs: vec![
            Pair::new('(', ')'),
            Pair::new('[', ']'),
            Pair::new('{', '}'),
            Pair::new('\'', '\''),
            Pair::new('"', '"'),
        ],
        match_pairs: Vec::new(),
        ..LanguageEditing::default()
    }
}

mod closer;
mod copy;
mod html;
mod language;
mod repeat;
mod sql;
