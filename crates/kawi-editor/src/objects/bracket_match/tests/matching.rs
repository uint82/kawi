use super::*;

#[test]
fn all_pairs_match_in_both_directions() {
    let buffer = Buffer::from_string("(a) [b] {c} <d>");
    let editing = rust_editing();
    for (index, _) in ALL_PAIRS.iter().enumerate() {
        let open_col = [0, 4, 8, 12][index];
        let close_col = [2, 6, 10, 14][index];
        assert_eq!(
            find(
                &buffer,
                &editing,
                None,
                &ALL_PAIRS,
                (0, open_col),
                ACTION_MATCH_BOUND
            ),
            Some((0, close_col))
        );
        assert_eq!(
            find(
                &buffer,
                &editing,
                None,
                &ALL_PAIRS,
                (0, close_col),
                ACTION_MATCH_BOUND
            ),
            Some((0, open_col))
        );
    }
}

#[test]
fn nested_pairs_match_inside_out() {
    let buffer = Buffer::from_string("a (b (c) d) e");
    let editing = rust_editing();
    assert_eq!(
        find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            (0, 5),
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
            (0, 2),
            ACTION_MATCH_BOUND
        ),
        Some((0, 10))
    );
    assert_eq!(
        find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            (0, 10),
            ACTION_MATCH_BOUND
        ),
        Some((0, 2))
    );
}

#[test]
fn multiline_pairs_match_in_both_directions() {
    let buffer = Buffer::from_string("call(\n  value,\n  other\n)");
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
        Some((3, 0))
    );
    assert_eq!(
        find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            (3, 0),
            ACTION_MATCH_BOUND
        ),
        Some((0, 4))
    );
}

#[test]
fn escaped_brackets_pair_with_matching_escapes() {
    let escaped = Buffer::from_string("\\(a\\)");
    let unescaped = Buffer::from_string("(a\\)b)");
    let editing = rust_editing();
    assert_eq!(
        find(
            &escaped,
            &editing,
            None,
            &ALL_PAIRS,
            (0, 1),
            ACTION_MATCH_BOUND
        ),
        Some((0, 4))
    );
    assert_eq!(
        find(
            &unescaped,
            &editing,
            None,
            &ALL_PAIRS,
            (0, 0),
            ACTION_MATCH_BOUND
        ),
        Some((0, 5))
    );
}

#[test]
fn unmatched_brackets_return_nothing() {
    let editing = rust_editing();
    for (text, position) in [("(", (0, 0)), ("a ) b", (0, 2)), ("(a [b c", (0, 0))] {
        let buffer = Buffer::from_string(text);
        assert!(find(
            &buffer,
            &editing,
            None,
            &ALL_PAIRS,
            position,
            ACTION_MATCH_BOUND
        )
        .is_none());
    }
}
