use std::collections::BTreeSet;

use tree_sitter::Query;

use super::capture_names::capture_names;
use super::{registry, SyntaxHighlight};
use crate::language::LanguageId;

#[test]
fn extracts_capture_names() {
    let names = capture_names("(identifier) @variable.member @function");
    assert_eq!(names, ["variable.member", "function"]);
}

#[test]
fn accepts_all_capture_name_characters() {
    let names = capture_names("@_name @name-with-dash @name.with.dot @1name @-name");
    assert_eq!(
        names,
        ["_name", "name-with-dash", "name.with.dot", "1name", "-name"]
    );
}

#[test]
fn ignores_capture_like_text_in_comments_and_strings() {
    let source = "; @commented\n(#match? \"@string\" \"x\")\n(identifier) @kept";
    assert_eq!(capture_names(source), ["kept"]);
}

#[test]
fn ignores_escaped_quotes_in_strings() {
    let source = "(#match? \"a\\\"@ignored\" \"x\") @kept";
    assert_eq!(capture_names(source), ["kept"]);
}

#[test]
fn resumes_after_unterminated_strings_at_line_breaks() {
    let source = "(#match? \"unterminated\n(identifier) @kept";
    assert_eq!(capture_names(source), ["kept"]);
}

#[test]
fn ignores_incomplete_capture_markers() {
    let source = "@ (identifier) @.name";
    assert!(capture_names(source).is_empty());
}

#[test]
fn stops_capture_names_at_delimiters() {
    let source = "(@name, @other)";
    assert_eq!(capture_names(source), ["name", "other"]);
}

#[test]
fn bundled_capture_names_match_compiled_queries() {
    for language in LanguageId::ALL {
        let source = language.bundled_highlights();
        let query = Query::new(&language.language(), &source).expect("bundled query");
        let expected = query
            .capture_names()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let actual = capture_names(&source).into_iter().collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{language:?}");
    }
}

#[test]
fn registry_names_are_sorted_and_indexable() {
    let registry = registry();
    assert!(registry.names().windows(2).all(|pair| pair[0] < pair[1]));
    for (index, name) in registry.names().iter().enumerate() {
        assert_eq!(
            registry.name(SyntaxHighlight(index as u16)),
            Some(name.as_str())
        );
        assert_eq!(registry.index_of(name), Some(SyntaxHighlight(index as u16)));
    }
}

#[test]
fn registry_resolves_parent_captures() {
    assert_eq!(
        registry().resolve("comment.documentation.extra"),
        registry().index_of("comment.documentation")
    );
}
