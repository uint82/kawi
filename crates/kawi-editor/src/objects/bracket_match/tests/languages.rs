use super::*;

#[test]
fn code_brackets_skip_strings_and_comments() {
    let text = "f(\"a)b\") # (comment)\n";
    let buffer = Buffer::from_string(text);
    let editing = python_editing();
    assert_eq!(
        find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            position_of(text, '(', 0),
            ACTION_MATCH_BOUND
        ),
        Some(position_of(text, ')', 1))
    );
    assert!(find(
        &buffer,
        &editing,
        None,
        &ALL_PAIRS,
        position_of(text, ')', 0),
        ACTION_MATCH_BOUND
    )
    .is_none());
    assert_eq!(
        find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            position_of(text, '(', 1),
            ACTION_MATCH_BOUND
        ),
        Some(position_of(text, ')', 2))
    );
}

#[test]
fn brackets_inside_strings_match_within_the_same_string() {
    let buffer = Buffer::from_string("f(\"a(b)c\")");
    let editing = rust_editing();
    assert_eq!(
        find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            (0, 4),
            ACTION_MATCH_BOUND
        ),
        Some((0, 6))
    );
    let result = find_result(
        &buffer,
        &editing,
        None,
        &ALL_PAIRS,
        (0, 4),
        ACTION_MATCH_BOUND,
    )
    .expect("string pair");
    assert!(!result.in_code);
    assert!(find(
        &buffer,
        &editing,
        None,
        &ALL_PAIRS,
        (0, 3),
        ACTION_MATCH_BOUND
    )
    .is_none());
}

#[test]
fn rust_char_literals_are_skipped_and_lifetimes_stay_code() {
    let text = "fn f<'a>(x: &'a str) { let c = '('; let d = (1); }\n";
    let buffer = Buffer::from_string(text);
    let editing = rust_editing();
    let lifetime_call = position_of(text, '(', 0);
    let character = position_of(text, '(', 1);
    let later_call = position_of(text, '(', 2);
    let later_close = position_of(text, ')', 1);
    assert_eq!(
        find(
            &buffer,
            &editing,
            Some(LanguageId::Rust),
            &ALL_PAIRS,
            lifetime_call,
            ACTION_MATCH_BOUND
        ),
        Some(position_of(text, ')', 0))
    );
    assert!(find(
        &buffer,
        &editing,
        Some(LanguageId::Rust),
        &ALL_PAIRS,
        character,
        ACTION_MATCH_BOUND
    )
    .is_none());
    assert_eq!(
        find(
            &buffer,
            &editing,
            Some(LanguageId::Rust),
            &ALL_PAIRS,
            later_call,
            ACTION_MATCH_BOUND
        ),
        Some(later_close)
    );
}

#[test]
fn bash_parameter_expansion_matches_as_braces() {
    let buffer = Buffer::from_string("x=${var} (y)");
    let editing = bash_editing();
    assert_eq!(
        find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            (0, 3),
            ACTION_MATCH_BOUND
        ),
        Some((0, 7))
    );
    assert_eq!(
        find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            (0, 9),
            ACTION_MATCH_BOUND
        ),
        Some((0, 11))
    );
}

#[test]
fn brackets_inside_block_comments_match_within_the_comment() {
    let text = "/* (a) */ code\n";
    let buffer = Buffer::from_string(text);
    let editing = rust_editing();
    assert_eq!(
        find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            (0, 3),
            ACTION_MATCH_BOUND
        ),
        Some((0, 5))
    );
}
