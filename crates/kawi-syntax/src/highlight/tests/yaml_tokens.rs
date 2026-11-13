use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_yaml_comments() {
    let text = "\
# top comment
name: value # inline comment
";
    let spans = highlight(LanguageId::Yaml, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "# top"), Some("comment"));
    assert_eq!(name_of(&spans, text, 1, "# inline"), Some("comment"));
}

#[test]
fn highlights_yaml_quoted_scalar_escapes() {
    let text = "\
double: \"line\\nbreak\"
single: 'it''s'
";
    let spans = highlight(LanguageId::Yaml, text, 0, 2);
    assert_eq!(name_of(&spans, text, 0, "line"), Some("string"));
    assert_eq!(name_of(&spans, text, 0, "\\n"), Some("string.escape"));
    assert_eq!(name_of(&spans, text, 1, "it"), Some("string"));
    assert_eq!(name_of(&spans, text, 1, "''"), Some("string.escape"));
}

#[test]
fn highlights_yaml_collection_punctuation() {
    let text = "\
key: [one, two]
? explicit
: {nested: value}
";
    let spans = highlight(LanguageId::Yaml, text, 0, 3);
    assert_eq!(name_of(&spans, text, 0, "["), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 0, "]"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 0, ","), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 1, "?"), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 2, "{"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 2, "nested"), Some("property"));
    assert_eq!(name_of(&spans, text, 2, "}"), Some("punctuation.bracket"));
}
