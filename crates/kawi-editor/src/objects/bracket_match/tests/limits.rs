use super::*;

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
