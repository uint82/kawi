use crate::editing::{BlockComment, CommentTokens, LanguageEditing, Pair};

use super::engine::{closer_width, next_width, IndentRequest, INDENT_WINDOW_LINES};
use super::{detect, IndentConfig, IndentRules};

fn editing() -> LanguageEditing {
    LanguageEditing {
        comments: CommentTokens {
            line: Some("#".to_string()),
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

fn python() -> IndentRules {
    IndentRules::new(vec![":".to_string()], Vec::new(), true)
}

fn bash() -> IndentRules {
    IndentRules::new(
        Vec::new(),
        vec!["then".to_string(), "do".to_string(), "{".to_string()],
        true,
    )
}

fn brackets() -> IndentRules {
    IndentRules::default()
}

fn width_at(
    lines: &[String],
    row: usize,
    col: usize,
    current_width: usize,
    rules: &IndentRules,
) -> usize {
    let start = row.saturating_sub(INDENT_WINDOW_LINES - 1);
    let window = &lines[start..=row];
    let editing = editing();
    let request = IndentRequest {
        lines: window,
        row: row - start,
        col,
        current_width,
        shiftwidth: 4,
        tabstop: 4,
        rules,
        editing: &editing,
    };
    next_width(&request)
}

fn width(lines: &[&str], current_width: usize, rules: &IndentRules) -> usize {
    let lines: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
    let row = lines.len().saturating_sub(1);
    let col = lines[row].chars().count();
    width_at(&lines, row, col, current_width, rules)
}

fn closer(lines: &[&str], closer: char) -> Option<usize> {
    let lines: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
    let row = lines.len().saturating_sub(1);
    let start = row.saturating_sub(INDENT_WINDOW_LINES - 1);
    let window = &lines[start..=row];
    let line = &lines[row];
    let col = line.chars().count() - line.trim_start().chars().count();
    let editing = editing();
    let request = IndentRequest {
        lines: window,
        row: row - start,
        col,
        current_width: col,
        shiftwidth: 4,
        tabstop: 4,
        rules: &brackets(),
        editing: &editing,
    };
    closer_width(&request, closer)
}

#[test]
fn python_block_suffixes_indent() {
    for line in [
        "class Solution:",
        "def run():",
        "async def run():",
        "if ready:",
        "else:",
        "try:",
        "finally:",
        "with open(path):",
        "for item in items:",
        "while ready:",
        "match value:",
    ] {
        assert_eq!(width(&[line], 0, &python()), 4, "{line}");
    }
}

#[test]
fn python_nested_blocks_keep_their_base() {
    assert_eq!(
        width(&["class Solution:", "    def run():"], 4, &python()),
        8
    );
}

#[test]
fn python_non_block_lines_keep_indent() {
    for line in [
        "return value",
        "x: int",
        "d[1:2]",
        "d = {\"a\": 1}",
        "lambda x: x",
        "# if ready:",
        "x = \":\"",
        "x = 1  # if ready:",
    ] {
        assert_eq!(width(&[line], 2, &python()), 2, "{line}");
    }
}

#[test]
fn python_comment_after_block_suffix_indents() {
    assert_eq!(width(&["if ready:  # keep going"], 0, &python()), 4);
}

#[test]
fn python_cursor_before_suffix_keeps_indent() {
    let lines = vec!["if ready: pass".to_string()];
    assert_eq!(width_at(&lines, 0, 2, 0, &python()), 0);
}

#[test]
fn whitespace_only_lines_keep_indent() {
    assert_eq!(width(&["class Solution:", "", "    "], 4, &python()), 4);
}

#[test]
fn hanging_brackets_indent_once_from_the_opener_line() {
    assert_eq!(width(&["foo(bar({"], 0, &brackets()), 4);
    assert_eq!(width(&["foo(", "    bar,", "    baz,"], 4, &brackets()), 4);
    assert_eq!(
        width(
            &["    foo(", "        bar(", "        baz,"],
            8,
            &brackets()
        ),
        8
    );
}

#[test]
fn block_rules_wait_for_bracket_depth_zero() {
    assert_eq!(width(&["d = {", "    \"a\":"], 4, &python()), 4);
    assert_eq!(width(&["items = [", "    lambda x:"], 4, &python()), 4);
}

#[test]
fn matched_brackets_keep_indent() {
    assert_eq!(width(&["foo(bar)"], 0, &brackets()), 0);
    assert_eq!(width(&["[table]"], 0, &brackets()), 0);
}

#[test]
fn leading_closer_and_trailing_opener_uses_remaining_stack() {
    assert_eq!(width(&["} else {"], 0, &brackets()), 4);
}

#[test]
fn bash_block_keywords_indent() {
    for line in ["then", "do", "if ready; then", "{"] {
        assert_eq!(width(&[line], 0, &bash()), 4, "{line}");
    }
}

#[test]
fn bash_parameter_expansion_is_not_a_block() {
    assert_eq!(width(&["echo ${value}"], 0, &bash()), 0);
    assert_eq!(width(&["echo ${value"], 0, &bash()), 0);
    assert_eq!(closer(&["echo ${value", "    }"], '}'), None);
}

#[test]
fn bracket_flag_disables_bracket_indent() {
    let rules = IndentRules::new(Vec::new(), Vec::new(), false);
    assert_eq!(width(&["foo("], 0, &rules), 0);
    assert_eq!(width(&["foo[", "    bar"], 4, &rules), 4);
}

#[test]
fn json_and_toml_use_brackets() {
    assert_eq!(width(&["["], 0, &brackets()), 4);
    assert_eq!(width(&["{\"key\": 1"], 0, &brackets()), 4);
    assert_eq!(width(&["[table]"], 0, &brackets()), 0);
}

#[test]
fn window_beyond_limit_falls_back_to_copy_indent() {
    let mut lines = vec!["foo(".to_string()];
    lines.extend((1..INDENT_WINDOW_LINES).map(|_| "x".to_string()));
    let inside = lines.len() - 1;
    assert_eq!(width_at(&lines, inside, 1, 8, &brackets()), 4);
    lines.push("x".to_string());
    let outside = lines.len() - 1;
    assert_eq!(width_at(&lines, outside, 1, 8, &brackets()), 8);
}

#[test]
fn closer_aligns_to_matching_opener_line() {
    assert_eq!(
        closer(&["    foo(", "        bar,", "        )"], ')'),
        Some(4)
    );
}

#[test]
fn closer_uses_opener_indent_after_manual_indent() {
    assert_eq!(
        closer(&["    foo(", "            bar,", "            )"], ')'),
        Some(4)
    );
}

#[test]
fn closer_ignores_mismatched_or_missing_openers() {
    assert_eq!(closer(&["foo(", "    }"], '}'), None);
    assert_eq!(closer(&["foo(bar)", "    ]"], ']'), None);
}

#[test]
fn closer_does_not_fire_inside_comments() {
    assert_eq!(closer(&["foo(", "/*", "    "], '}'), None);
}

#[test]
fn closer_outside_window_is_ignored() {
    let mut lines = vec!["foo(".to_string()];
    lines.extend((1..INDENT_WINDOW_LINES).map(|_| "x".to_string()));
    lines.push("    ".to_string());
    let row = lines.len() - 1;
    let start = row.saturating_sub(INDENT_WINDOW_LINES - 1);
    let window = &lines[start..=row];
    let editing = editing();
    let request = IndentRequest {
        lines: window,
        row: row - start,
        col: 4,
        current_width: 4,
        shiftwidth: 4,
        tabstop: 4,
        rules: &brackets(),
        editing: &editing,
    };
    assert_eq!(closer_width(&request, ')'), None);
}

#[test]
fn detects_language_defaults() {
    assert_eq!(detect(Some("x.py")), IndentConfig::filetype(4, true, 4));
    assert_eq!(detect(Some("x.rs")), IndentConfig::filetype(4, true, 8));
    assert_eq!(detect(Some("x.zig")), IndentConfig::filetype(4, true, 8));
    assert_eq!(detect(Some("x.swift")), IndentConfig::filetype(4, true, 8));
    assert_eq!(detect(Some("x.go")), IndentConfig::filetype(0, false, 8));
    assert_eq!(detect(Some("x.yaml")), IndentConfig::filetype(2, true, 8));
    assert_eq!(detect(Some("x.yml")), IndentConfig::filetype(2, true, 8));
    assert_eq!(detect(Some("x.rb")), IndentConfig::filetype(2, true, 8));
    assert_eq!(detect(Some("x.scala")), IndentConfig::filetype(2, true, 8));
    assert_eq!(detect(Some("x.ex")), IndentConfig::filetype(2, true, 8));
    assert_eq!(detect(Some("x.tf")), IndentConfig::filetype(2, true, 2));
    assert_eq!(detect(Some("x.ml")), IndentConfig::filetype(8, true, 8));
    assert_eq!(detect(Some("x.md")), IndentConfig::filetype(4, true, 4));
    assert_eq!(
        detect(Some("x.markdown")),
        IndentConfig::filetype(4, true, 4)
    );
}

#[test]
fn falls_back_without_extension() {
    assert_eq!(detect(None), IndentConfig::fallback());
    assert_eq!(detect(Some("Makefile")), IndentConfig::fallback());
    assert_eq!(detect(Some("x.js")), IndentConfig::fallback());
    assert_eq!(detect(Some("X.PY")), IndentConfig::filetype(4, true, 4));
}
