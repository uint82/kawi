use super::engine::{tag_closer_width, IndentRequest, INDENT_WINDOW_LINES};
use super::tests::{editing, width};
use super::IndentRules;

fn html() -> IndentRules {
    IndentRules::default().with_indent_tags(true)
}

fn tag_closer(lines: &[&str], col: usize) -> Option<usize> {
    let lines: Vec<String> = lines.iter().map(|line| (*line).to_string()).collect();
    let row = lines.len().saturating_sub(1);
    let start = row.saturating_sub(INDENT_WINDOW_LINES - 1);
    let window = &lines[start..=row];
    let editing = editing();
    let request = IndentRequest {
        lines: window,
        row: row - start,
        col,
        current_width: 0,
        shiftwidth: 4,
        tabstop: 4,
        rules: &html(),
        editing: &editing,
        language: None,
    };
    tag_closer_width(&request)
}

#[test]
fn html_opening_tags_indent() {
    for line in ["<div>", "<section class=\"a\">", "<custom-element>"] {
        assert_eq!(width(&[line], 0, &html()), 4, "{line}");
    }
}

#[test]
fn html_nested_tags_indent_from_the_innermost_tag() {
    assert_eq!(width(&["<div>", "    <p>"], 4, &html()), 8);
}

#[test]
fn html_closing_tags_return_to_the_parent_body() {
    assert_eq!(
        width(&["<div>", "    <p>", "        text</p>"], 8, &html()),
        4
    );
    assert_eq!(width(&["<div>", "    <p>", "    </p>"], 4, &html()), 4);
    assert_eq!(width(&["<div>", "    </div>"], 0, &html()), 0);
}

#[test]
fn html_void_and_self_closing_tags_do_not_indent() {
    for line in [
        "<br>",
        "<img src=\"logo.png\">",
        "<input type=\"text\">",
        "<meta charset=\"utf-8\">",
        "<hr />",
        "<custom-element />",
    ] {
        assert_eq!(width(&[line], 0, &html()), 0, "{line}");
    }
}

#[test]
fn html_balanced_tags_do_not_indent() {
    assert_eq!(width(&["<div></div>"], 0, &html()), 0);
    assert_eq!(width(&["<div><span>text</span>"], 0, &html()), 4);
}

#[test]
fn html_declarations_and_comments_do_not_indent() {
    assert_eq!(width(&["<!DOCTYPE html>"], 0, &html()), 0);
    assert_eq!(width(&["<!-- note -->"], 0, &html()), 0);
    assert_eq!(width(&["<?xml version=\"1.0\"?>"], 0, &html()), 0);
}

#[test]
fn html_multiline_tags_indent_attributes() {
    assert_eq!(width(&["<div", "    class=\"a\""], 4, &html()), 4);
    assert_eq!(
        width(&["<div", "    class=\"a\"", "    id=\"b\""], 4, &html()),
        4
    );
}

#[test]
fn html_script_comparisons_do_not_start_a_tag() {
    assert_eq!(
        width(&["<script>", "    if (a<b && c>d) return;"], 4, &html()),
        4
    );
}

#[test]
fn html_script_contents_use_brackets() {
    assert_eq!(
        width(
            &["<script>", "    if (ready) {", "        run();"],
            8,
            &html()
        ),
        8
    );
}

#[test]
fn html_closing_tag_aligns_to_matching_opener() {
    assert_eq!(tag_closer(&["<div>", "    <p>", "    </p"], 7), Some(4));
    assert_eq!(
        tag_closer(&["<div>", "    <span>", "    </span"], 10),
        Some(4)
    );
}

#[test]
fn html_closing_tag_ignores_unknown_tags() {
    assert_eq!(tag_closer(&["<div>", "    </span"], 10), None);
}

#[test]
fn html_tag_attribute_closer_dedents_to_tag_start() {
    assert_eq!(tag_closer(&["<div", "    "], 4), Some(0));
}

#[test]
fn html_tag_closer_ignores_non_tag_prefixes() {
    assert_eq!(tag_closer(&["<div>", "    text"], 8), None);
}
