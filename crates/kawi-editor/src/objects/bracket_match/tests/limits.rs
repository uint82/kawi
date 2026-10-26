use super::*;

fn long_block_comment_code() -> String {
    let mut text = String::from("/*\n");
    for _ in 0..10_000 {
        text.push_str("xxxxxxx\n");
    }
    text.push_str("*/\n(a)\n");
    text
}

fn long_triple_string_code() -> String {
    let mut text = String::from("\"\"\"\n");
    for _ in 0..10_000 {
        text.push_str("xxxxxxx\n");
    }
    text.push_str("\"\"\"\n(a)\n");
    text
}

#[test]
fn search_respects_the_bound() {
    let buffer = Buffer::from_string("a (xxxxx)");
    let editing = rust_editing();
    assert!(find(&buffer, &editing, None, &ALL_PAIRS, (0, 2), 5).is_none());
    assert_eq!(
        find(&buffer, &editing, None, &ALL_PAIRS, (0, 2), 6),
        Some((0, 8))
    );
}

#[test]
fn very_long_lines_skip_matching() {
    let mut text = String::from("(");
    text.push_str(&"x".repeat(MAX_MATCH_LINE_CHARS));
    text.push(')');
    let buffer = Buffer::from_string(&text);
    let editing = rust_editing();
    assert!(find(
        &buffer,
        &editing,
        None,
        &ALL_PAIRS,
        (0, 0),
        ACTION_MATCH_BOUND
    )
    .is_none());
}

#[test]
fn code_after_a_block_comment_is_matched() {
    let text = format!("/*\n{}\n*/\ncall(value)\n", "(".repeat(100));
    let buffer = Buffer::from_string(&text);
    let editing = rust_editing();
    let call = position_of(&text, '(', 100);
    let close = position_of(&text, ')', 0);
    assert_eq!(
        find(
            &buffer,
            &editing,
            Some(LanguageId::Rust),
            &ALL_PAIRS,
            call,
            ACTION_MATCH_BOUND
        ),
        Some(close)
    );
}

#[test]
fn bounded_search_skips_large_files() {
    let mut text = String::from("(\n");
    for _ in 0..10_000 {
        text.push_str("xxxxxxx\n");
    }
    text.push(')');
    let buffer = Buffer::from_string(&text);
    let editing = rust_editing();
    assert!(find(&buffer, &editing, None, &ALL_PAIRS, (0, 0), 100).is_none());
}

#[test]
fn context_inside_block_comment_recovers_after_the_comment() {
    let text = long_block_comment_code();
    let buffer = Buffer::from_string(&text);
    let editing = rust_editing();
    assert_eq!(
        find(
            &buffer,
            &editing,
            Some(LanguageId::Rust),
            &ALL_PAIRS,
            position_of(&text, '(', 0),
            10,
        ),
        Some(position_of(&text, ')', 0))
    );
}

#[test]
fn context_inside_triple_string_recovers_after_the_string() {
    let text = long_triple_string_code();
    let buffer = Buffer::from_string(&text);
    let editing = python_editing();
    assert_eq!(
        find(
            &buffer,
            &editing,
            Some(LanguageId::Python),
            &ALL_PAIRS,
            position_of(&text, '(', 0),
            10,
        ),
        Some(position_of(&text, ')', 0))
    );
}
