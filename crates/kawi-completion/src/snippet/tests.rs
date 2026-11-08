use std::ops::Range;

use super::Snippet;

fn parse(source: &str) -> Snippet {
    Snippet::parse(source)
}

fn text(source: &str) -> String {
    parse(source).text().to_string()
}

fn choice_strings(snippet: &Snippet) -> Vec<&str> {
    snippet.stops()[0]
        .choices()
        .iter()
        .map(String::as_str)
        .collect()
}

fn stops(source: &str) -> Vec<(usize, Range<usize>)> {
    parse(source)
        .stops()
        .iter()
        .map(|stop| (stop.index(), stop.range()))
        .collect()
}

#[test]
fn plain_text_is_unchanged() {
    let snippet = parse("hello");
    assert_eq!(snippet.text(), "hello");
    assert!(snippet.stops().is_empty());
    assert_eq!(snippet.final_stop(), 5);
}

#[test]
fn tabstops_are_removed_and_recorded() {
    assert_eq!(text("fn $1()"), "fn ()");
    assert_eq!(stops("fn $1()"), vec![(1, 3..3)]);
}

#[test]
fn placeholder_keeps_default() {
    assert_eq!(text("let ${1:name} = $2;"), "let name = ;");
    assert_eq!(stops("let ${1:name} = $2;"), vec![(1, 4..8), (2, 11..11)]);
}

#[test]
fn zero_sets_the_final_position() {
    let snippet = parse("abc$0def");
    assert_eq!(snippet.text(), "abcdef");
    assert_eq!(snippet.final_stop(), 3);
    assert!(snippet.stops().is_empty());
}

#[test]
fn choices_insert_the_first_option() {
    let snippet = parse("${1|one,two|}");
    assert_eq!(snippet.text(), "one");
    assert_eq!(snippet.stops()[0].range(), 0..3);
    assert_eq!(choice_strings(&snippet), vec!["one", "two"]);
}

#[test]
fn choice_escapes_are_unescaped() {
    let snippet = parse(r"${1|a\,b,c\|d|}");
    assert_eq!(snippet.text(), "a,b");
    assert_eq!(choice_strings(&snippet), vec!["a,b", "c|d"]);
}

#[test]
fn escaped_dollar_is_literal() {
    assert_eq!(text(r"\$1"), "$1");
    assert!(stops(r"\$1").is_empty());
}

#[test]
fn literal_dollars_are_preserved() {
    assert_eq!(text("cost $"), "cost $");
    assert_eq!(text("cost $$"), "cost $$");
}

#[test]
fn escaped_brace_is_literal() {
    assert_eq!(text(r"\}"), "}");
}

#[test]
fn trailing_backslash_is_literal() {
    assert_eq!(text("\\"), "\\");
}

#[test]
fn variable_defaults_are_inserted() {
    assert_eq!(text("${TM_FILENAME:file.rs}"), "file.rs");
    assert!(stops("${TM_FILENAME:file.rs}").is_empty());
}

#[test]
fn variables_without_defaults_are_removed() {
    assert_eq!(text("$TM_FILENAME.ts"), ".ts");
}

#[test]
fn nested_placeholders_record_both_stops() {
    assert_eq!(text("${1:outer ${2:inner}}"), "outer inner");
    assert_eq!(stops("${1:outer ${2:inner}}"), vec![(1, 0..11), (2, 6..11)]);
}

#[test]
fn out_of_order_stops_are_sorted() {
    assert_eq!(stops("$2 $1"), vec![(1, 1..1), (2, 0..0)]);
}

#[test]
fn duplicate_stops_are_recorded_once() {
    assert_eq!(stops("$1$1"), vec![(1, 0..0)]);
}

#[test]
fn malformed_placeholders_are_preserved() {
    assert_eq!(text("${1"), "${1");
    assert_eq!(text("${1x"), "${1x");
    assert_eq!(text("${:x"), "${:x");
}

#[test]
fn placeholder_transforms_are_ignored() {
    assert_eq!(text("a${1/(x)/[$1]/}b"), "ab");
    assert_eq!(stops("a${1/(x)/[$1]/}b"), vec![(1, 1..1)]);
}

#[test]
fn variable_transforms_are_ignored() {
    assert_eq!(text("${TM_FILENAME/(.*)/$1/}"), "");
    assert_eq!(text("$TM_FILENAME.ts"), ".ts");
    assert_eq!(text("${TM_FILENAME/a{2}/x/}"), "");
}

#[test]
fn unicode_offsets_are_character_based() {
    assert_eq!(text("é${1:x}é"), "éxé");
    assert_eq!(stops("é${1:x}é"), vec![(1, 1..2)]);
}

#[test]
fn final_stop_can_be_inside_a_placeholder() {
    let snippet = parse("${1:a$0b}");
    assert_eq!(snippet.text(), "ab");
    assert_eq!(snippet.final_stop(), 1);
    assert_eq!(snippet.stops()[0].range(), 0..2);
}

#[test]
fn empty_choice_records_an_empty_stop() {
    let snippet = parse("${1||}");
    assert_eq!(snippet.text(), "");
    assert_eq!(snippet.stops()[0].range(), 0..0);
    assert_eq!(choice_strings(&snippet), vec![""]);
}
