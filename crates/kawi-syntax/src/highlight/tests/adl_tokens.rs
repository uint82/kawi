use super::support::{highlight, name_of};
use super::*;

#[test]
fn highlights_adl_keywords_operators_and_punctuation() {
    let text = "module demo {\n  type Name = String;\n  newtype Id = String = \"x\";\n  struct Pair<T1, T2> { T1 first; };\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 5);
    assert_eq!(name_of(&spans, text, 0, "module"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 1, "type"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 2, "newtype"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 3, "struct"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 1, "="), Some("operator"));
    assert_eq!(name_of(&spans, text, 3, "<"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 3, ">"), Some("punctuation.bracket"));
    assert_eq!(name_of(&spans, text, 1, ";"), Some("punctuation.delimiter"));
    assert_eq!(name_of(&spans, text, 3, ","), Some("punctuation.delimiter"));
}

#[test]
fn highlights_adl_json_literals_and_object_keys() {
    let text = "module demo {\nstruct Config\n{\n  Json data = {\n    \"name\": \"kawi\",\n    \"count\": 42,\n    \"ratio\": -1.5e3,\n    \"enabled\": true,\n    \"missing\": null,\n    \"items\": [\"one\", \"two\"]\n  };\n};\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 13);
    assert_eq!(name_of(&spans, text, 4, "\"name\""), Some("property"));
    assert_eq!(name_of(&spans, text, 4, "\"kawi\""), Some("string"));
    assert_eq!(name_of(&spans, text, 5, "42"), Some("number"));
    assert_eq!(name_of(&spans, text, 6, "-1.5e3"), Some("number"));
    assert_eq!(name_of(&spans, text, 7, "true"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 8, "null"), Some("constant.builtin"));
    assert_eq!(name_of(&spans, text, 9, "\"items\""), Some("property"));
    assert_eq!(name_of(&spans, text, 9, "\"one\""), Some("string"));
}

#[test]
fn highlights_adl_annotation_declaration_targets() {
    let text = "module demo {\nannotation Address.id DbField \"id\";\n};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 3);
    assert_eq!(name_of(&spans, text, 1, "annotation"), Some("keyword"));
    assert_eq!(name_of(&spans, text, 1, "Address.id"), Some("type"));
    assert_eq!(name_of(&spans, text, 1, "DbField"), Some("attribute"));
    assert_eq!(name_of(&spans, text, 1, "\"id\""), Some("string"));
}

#[test]
fn adl_docstring_keywords_are_not_highlighted_as_keywords() {
    let text = "/// module struct union type newtype annotation\nmodule demo {};\n";
    let spans = highlight(LanguageId::Adl, text, 0, 2);
    assert_eq!(
        name_of(&spans, text, 0, "module struct union"),
        Some("comment.documentation")
    );
    assert_eq!(name_of(&spans, text, 1, "module"), Some("keyword"));
}
